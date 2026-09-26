//! Worked example: size a battery for peak shaving from a load CSV.
//!
//! Run with:
//! `cargo run --manifest-path examples/Cargo.toml --bin battery-sizing`
//!
//! The task this answers: *"here is a day of metered load. How big a battery
//! do I need to shave my demand charge by X MW, and what will it cost?"*
//!
//! It walks five steps:
//!
//! 1. **Read** a metered load profile (`timestamp_utc,load_mw`) from CSV.
//! 2. **Characterise** it: peak, load factor, and where the peak sits.
//! 3. **Size** it, in two passes. A discovery pass dispatches a battery whose
//!    energy never binds, and the nameplate then follows from the SoC swing
//!    that pass observes. The energy is an *output* of the simulation.
//! 4. **Validate** the size against the real
//!    [`tpt_nrg_battery::BatteryStorage`] model, which enforces round-trip
//!    efficiency, the power rating, and the SoC floor.
//! 5. **Cost it out** with [`tpt_nrg_lcoe`] and report payback against
//!    avoided demand charges.
//!
//! Steps 3 and 4 are deliberately separate. The rule of thumb "energy =
//! reduction x window hours" ignores that the battery must refill before it
//! can shave again, cannot discharge below its floor, and loses energy to
//! round-trip losses. Those effects decide whether a quoted size works, so the
//! size is found by simulating them and then confirmed against the real model
//! rather than asserted.
//!
//! The worked result is instructive rather than flattering: this profile has a
//! 0.82 load factor, so almost every hour sits above the peak-shaving ceiling.
//! Holding a flat ceiling therefore demands a *large* battery, and the honest
//! conclusion is that peak shaving is a poor investment against energy prices.
//! The example reports that rather than hiding it behind a flattering size.

use std::path::{Path, PathBuf};

use tpt_nrg_battery::{BatteryStorage, DegradationModel};
use tpt_nrg_lcoe::{levelized_cost_of_energy, LcoeInputs};

/// Target demand reduction, in MW.
const TARGET_MW: f64 = 8.0;
/// Round-trip efficiency of the assumed chemistry.
const ROUND_TRIP: f64 = 0.90;
/// The battery will not discharge below this state of charge.
const MIN_SOC: f64 = 0.10;
/// Capital cost, in dollars per kWh of nameplate capacity.
const CAPEX_PER_KWH: f64 = 250.0;
/// Annual fixed O&M, as a fraction of CAPEX.
const FIXED_OM_FRACTION: f64 = 0.015;
/// Project lifetime, in years.
const LIFETIME_YEARS: u32 = 15;
/// Discount rate.
const DISCOUNT_RATE: f64 = 0.07;
/// Days assumed per year when scaling a one-day profile up.
const DAYS_PER_YEAR: f64 = 365.0;
/// Cost of one MW of peak, in dollars per year.
const PEAK_CHARGE_PER_MW_YEAR: f64 = 18_000.0;
/// Spacing of the committed fixtures, in hours.
const INTERVAL_H: f64 = 0.25;
/// Nominal capacity for the discovery pass, in MWh.
///
/// Large enough that the SoC floor never binds, so the pass measures the duty
/// cycle's natural energy swing rather than an arbitrary energy limit. Only
/// the fraction of this capacity that gets used carries meaning.
const DISCOVERY_CAPACITY_MWH: f64 = 1_000.0;
/// Margin applied to the discovered swing, for refill and degradation.
const SIZE_MARGIN: f64 = 1.25;
/// Shortest useful discharge window, in hours.
const MIN_WINDOW_H: f64 = 0.5;

/// A candidate battery size and how it performed on the profile.
#[derive(Debug, Clone)]
struct Candidate {
    /// Power rating, in MW.
    power_mw: f64,
    /// Nameplate energy, in MWh.
    energy_mwh: f64,
    /// Reduction at the system peak, in MW.
    shaved_peak_mw: f64,
    /// Equivalent full cycles over the profile.
    cycles: f64,
    /// Whether it met the target reduction.
    meets_target: bool,
}

/// Every candidate in the sweep, plus the chosen one.
struct Sweep {
    /// All candidates, in ascending power order.
    candidates: Vec<Candidate>,
    /// The smallest that met the target.
    chosen: Candidate,
}

/// What the real SoC model says about a chosen size.
struct Validated {
    /// Hours the battery can hold full power from full to the floor.
    usable_hours: f64,
    /// Lowest state of charge reached, as a fraction.
    min_soc: f64,
    /// Equivalent full cycles accumulated over the profile.
    cycles: f64,
    /// Cycle-driven capacity fade, in percent.
    cycle_fade_percent: f64,
    /// Energy actually delivered, in MWh.
    delivered_mwh: f64,
    /// Reduction at the system peak, in MW, as delivered.
    shaved_peak_mw: f64,
}

/// Outcome of dispatching one battery size over the profile.
struct Run {
    /// Delivered power per interval, in MW, positive when discharging.
    power_mw: Vec<f64>,
    /// Lowest state of charge reached, as a fraction.
    min_soc: f64,
    /// Highest state of charge reached, as a fraction.
    max_soc: f64,
    /// Equivalent full cycles accumulated.
    cycles: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Sizing a battery for peak shaving ===\n");

    // ----------------------------------------------------------------
    // 1. Read the load profile
    // ----------------------------------------------------------------
    let path = repo_path("test-data/load-profiles/commercial-winter-24h.csv")?;
    let profile = read_load_csv(&path)?;
    let hours = span_hours(&profile);
    println!("Step 1: Load profile");
    println!("  file: {}", path.display());
    println!("  {} intervals, {hours:.2} h span", profile.len());
    println!(
        "  energy over the window: {:.1} MWh",
        profile.iter().sum::<f64>()
    );

    // ----------------------------------------------------------------
    // 2. Characterise
    // ----------------------------------------------------------------
    let peak_index = peak_index(&profile);
    let peak = profile[peak_index];
    let mean = profile.iter().sum::<f64>() / profile.len() as f64;
    println!("\nStep 2: Characterisation");
    println!(
        "  peak        : {peak:.2} MW at hour {:.2}",
        peak_index as f64 * INTERVAL_H
    );
    println!("  mean        : {mean:.2} MW");
    println!("  load factor : {:.2}", mean / peak);

    // ----------------------------------------------------------------
    // 3. Size
    // ----------------------------------------------------------------
    let ceiling = peak - TARGET_MW;
    let share_above = profile.iter().filter(|&&l| l > ceiling).count() as f64 * INTERVAL_H / hours;
    println!("\nStep 3: Sizing sweep (target {TARGET_MW:.1} MW reduction)");
    println!(
        "  the {ceiling:.2} MW ceiling sits below the load {:.0}% of the day, so the\n  \
         battery must refill and shave repeatedly, not once.",
        share_above * 100.0
    );
    println!(
        "  {:>10}  {:>10}  {:>12}  {:>10}  meets target",
        "power MW", "energy MWh", "shaved MW", "EFC"
    );
    println!("  {}", "-".repeat(62));

    let sweep = sweep(&profile, peak_index);
    for c in &sweep.candidates {
        println!(
            "  {:>10.1}  {:>10.2}  {:>12.2}  {:>10.2}  {}",
            c.power_mw,
            c.energy_mwh,
            c.shaved_peak_mw,
            c.cycles,
            if c.meets_target { "yes" } else { "no" }
        );
    }
    let chosen = sweep.chosen.clone();
    println!(
        "\n  smallest size holding the target: {:.1} MW / {:.1} MWh",
        chosen.power_mw, chosen.energy_mwh
    );
    println!(
        "  the power rating sets the reduction; the {:.0} MWh pack is needed only\n  \
         because the flat ceiling is cleared so often.",
        chosen.energy_mwh
    );

    // ----------------------------------------------------------------
    // 4. Validate against the real SoC model
    // ----------------------------------------------------------------
    println!("\nStep 4: Validation with the real SoC model");
    let v = validate(&profile, peak_index, &chosen);
    println!("  power rating      : {:.1} MW", chosen.power_mw);
    println!("  nameplate energy  : {:.1} MWh", chosen.energy_mwh);
    println!(
        "  usable window     : {:.1} h at full power",
        v.usable_hours
    );
    println!(
        "  min SoC reached   : {:.1}% (floor {:.0}%)",
        v.min_soc * 100.0,
        MIN_SOC * 100.0
    );
    println!(
        "  shaved at peak    : {:.2} MW (target {:.1} MW)",
        v.shaved_peak_mw, TARGET_MW
    );
    println!(
        "  energy delivered  : {:.1} MWh over {:.1} h",
        v.delivered_mwh, hours
    );
    println!("  equivalent cycles : {:.2}", v.cycles);
    println!(
        "  cycle fade        : {:.2}% at this duty over {LIFETIME_YEARS} y",
        v.cycle_fade_percent
    );
    if v.shaved_peak_mw < TARGET_MW - 1e-6 {
        println!(
            "  WARNING: the real model shaves {:.2} MW short. Add margin.",
            TARGET_MW - v.shaved_peak_mw
        );
    } else {
        println!("  the real model holds the target through the peak window");
    }

    // ----------------------------------------------------------------
    // 5. Cost
    // ----------------------------------------------------------------
    let capex = chosen.energy_mwh * 1000.0 * CAPEX_PER_KWH;
    let fixed_om = capex * FIXED_OM_FRACTION;
    // A battery moves energy, it does not generate it. LCOE over the site
    // load would divide the capital by a number the battery never touches and
    // report a meaningless fraction of a cent per MWh, so the denominator is
    // the annual throughput the battery is actually responsible for.
    let annual_throughput = v.delivered_mwh * DAYS_PER_YEAR / 24.0;
    let lcoe = levelized_cost_of_energy(&LcoeInputs::new(
        capex,
        fixed_om,
        0.0,
        0.0,
        annual_throughput,
        LIFETIME_YEARS,
        DISCOUNT_RATE,
        0.0,
    ));
    let annual_avoided = chosen.power_mw * PEAK_CHARGE_PER_MW_YEAR;
    let payback = capex / annual_avoided;

    println!("\nStep 5: Cost");
    println!("  CAPEX                : ${capex:.0} (${CAPEX_PER_KWH:.0}/kWh)");
    println!("  annual fixed O&M     : ${fixed_om:.0}");
    println!("  annual throughput    : {annual_throughput:.0} MWh moved");
    println!(
        "  battery LCOE         : ${lcoe:.0}/MWh (lifetime {LIFETIME_YEARS} y, discount {:.0}%)",
        DISCOUNT_RATE * 100.0
    );
    println!("  demand charge avoided: ${annual_avoided:.0}/year");
    println!("  simple payback       : {payback:.0} years");

    // Report the conclusion the numbers actually support, rather than
    // implying the project is a good idea either way.
    println!("\n  Conclusion");
    if payback > (LIFETIME_YEARS as f64) {
        println!(
            "    Payback exceeds the {LIFETIME_YEARS}-year asset life: at this load\n    \
             factor, peak shaving does not pay for itself. Either the demand\n    \
             charge must be higher, or a smaller reduction ({:.0} MW) will not need\n    \
             the {:.0} MWh pack that holding a flat ceiling demands.",
            TARGET_MW / 2.0,
            chosen.energy_mwh
        );
    } else {
        println!("    Payback is inside the asset life; the sizing holds up.");
    }
    println!(
        "    Energy is worth {lcoe:.0}/MWh here, so arbitrage against a typical\n    \
         energy price would lose money; this asset only makes sense if the\n    \
         demand charge is the binding cost."
    );

    Ok(())
}

/// Read a `timestamp_utc,load_mw` CSV into a load profile in MW.
///
/// The timestamp column is part of the committed schema but the sizing only
/// needs the MW series, so the header is validated and the column ignored.
fn read_load_csv(path: &Path) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)?;
    let mut rows = text.lines();
    let header = rows
        .next()
        .ok_or_else(|| format!("{} is empty", path.display()))?
        .to_ascii_lowercase();
    if !header.contains("load_mw") {
        return Err(format!("{} has no `load_mw` column", path.display()).into());
    }
    let mut profile = Vec::new();
    for (n, line) in rows.enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let value: f64 = line
            .rsplit(',')
            .next()
            .and_then(|v| v.trim().parse().ok())
            .ok_or_else(|| format!("{}:{}: cannot parse load", path.display(), n + 2))?;
        if value < 0.0 {
            return Err(format!("{}:{}: negative load", path.display(), n + 2).into());
        }
        profile.push(value);
    }
    if profile.is_empty() {
        return Err(format!("{} has no data rows", path.display()).into());
    }
    Ok(profile)
}

/// Span of the profile in hours, from the interval count.
fn span_hours(profile: &[f64]) -> f64 {
    profile.len() as f64 * INTERVAL_H
}

/// Index of the highest-load interval.
fn peak_index(profile: &[f64]) -> usize {
    profile
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map_or(0, |(i, _)| i)
}

/// Path to a file in the repository, resolved from the examples workspace.
fn repo_path(relative: &str) -> Result<PathBuf, std::io::Error> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(relative);
    if path.is_file() {
        Ok(path)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} not found", path.display()),
        ))
    }
}

/// Sweep candidate power ratings, sizing the energy from the duty cycle.
///
/// Two passes per candidate:
///
/// 1. **Discovery**: dispatch a battery whose energy never binds and record how
///    far the SoC swings. That swing is the energy the profile genuinely
///    needs, so the nameplate follows from it instead of from a rule of thumb.
/// 2. **Nameplate**: convert the swing into a real capacity, grossed up for
///    the SoC floor, the round-trip losses, and a refill margin, then
///    re-dispatch at that capacity to confirm the target is actually met.
fn sweep(profile: &[f64], peak_index: usize) -> Sweep {
    let peak = profile[peak_index];
    let ceiling = peak - TARGET_MW;
    let mut candidates = Vec::new();

    for power_mw in [2.0_f64, 4.0, 6.0, 8.0, 10.0] {
        // Pass 1: discover the swing with a non-binding energy capacity.
        let discovery = run(profile, ceiling, power_mw, DISCOVERY_CAPACITY_MWH);
        let swing_fraction = (discovery.max_soc - discovery.min_soc).max(0.0);
        let usable_mwh = swing_fraction * DISCOVERY_CAPACITY_MWH;

        // Nameplate from the swing: only (1 - MIN_SOC) of the pack is usable,
        // the round trip means it must be larger still, and the margin covers
        // the window the battery cannot refill into.
        let energy_mwh =
            (usable_mwh / (1.0 - MIN_SOC) / ROUND_TRIP * SIZE_MARGIN).max(power_mw * MIN_WINDOW_H);

        // Pass 2: confirm at the real nameplate.
        let confirmed = run(profile, ceiling, power_mw, energy_mwh);
        candidates.push(Candidate {
            power_mw,
            energy_mwh,
            shaved_peak_mw: confirmed.power_mw[peak_index],
            cycles: confirmed.cycles,
            meets_target: confirmed.power_mw[peak_index] >= TARGET_MW - 1e-6,
        });
    }

    // Capital cost tracks kWh and the power electronics, so among the sizes
    // that meet the target take the least energy, breaking ties on power.
    let chosen = candidates
        .iter()
        .filter(|c| c.meets_target)
        .min_by(|a, b| {
            a.energy_mwh
                .total_cmp(&b.energy_mwh)
                .then(a.power_mw.total_cmp(&b.power_mw))
        })
        .cloned()
        .unwrap_or_else(|| candidates[candidates.len() - 1].clone());

    Sweep { candidates, chosen }
}

/// Dispatch one battery size greedily over the profile.
///
/// The policy is deliberately simple and physically honest:
///
/// * Above the ceiling, discharge to hold the load down, limited by the power
///   rating and by the energy above the floor.
/// * Below the ceiling, charge from the headroom, limited by the power rating
///   and by the room left to full.
/// * At the ceiling, do nothing.
///
/// Discharge takes priority over charging on purpose: a battery that charges
/// into a peak it could have shaved is worse than useless.
fn run(profile: &[f64], ceiling: f64, power_mw: f64, energy_mwh: f64) -> Run {
    let eta = ROUND_TRIP.sqrt();
    let mut soc = 1.0_f64;
    let mut min_soc = 1.0_f64;
    let mut max_soc = 1.0_f64;
    let mut cycles = 0.0_f64;
    let mut power = Vec::with_capacity(profile.len());

    for &load in profile {
        let p = if load > ceiling {
            // Energy available above the floor, grossed up for the discharge
            // leg, converted to a power that lasts one interval.
            let headroom_mwh = ((soc - MIN_SOC) * energy_mwh * eta).max(0.0);
            (load - ceiling)
                .min(power_mw)
                .min(headroom_mwh / INTERVAL_H)
        } else if load < ceiling {
            // Room above the current SoC, and the power it can absorb.
            let room_mwh = (1.0 - soc) * energy_mwh;
            -((ceiling - load).min(power_mw) * eta).min(room_mwh / INTERVAL_H)
        } else {
            0.0
        };

        let next = (soc - p * INTERVAL_H / eta / energy_mwh).clamp(MIN_SOC, 1.0);
        cycles += (next - soc).abs();
        soc = next;
        min_soc = min_soc.min(soc);
        max_soc = max_soc.max(soc);
        power.push(p);
    }

    Run {
        power_mw: power,
        min_soc,
        max_soc,
        cycles,
    }
}

/// Replay the chosen size through the real [`BatteryStorage`] model.
///
/// The sweep uses a simple closed-form dispatch. This pass uses the crate's
/// own SoC model so round-trip efficiency, the power rating, the SoC floor,
/// and cumulative throughput are *enforced* rather than assumed. Where the
/// two disagree, this is the number to believe.
fn validate(profile: &[f64], peak_index: usize, chosen: &Candidate) -> Validated {
    let peak = profile[peak_index];
    let ceiling = peak - TARGET_MW;

    let mut battery = BatteryStorage::new(chosen.energy_mwh, chosen.power_mw, ROUND_TRIP)
        .with_soc(MIN_SOC, 1.0)
        .with_degradation(DegradationModel::li_ion());
    let mut min_soc = battery.soc;
    let mut delivered = 0.0_f64;
    let mut shaved_at_peak = 0.0_f64;

    for (i, &load) in profile.iter().enumerate() {
        if load > ceiling {
            let want = (load - ceiling).min(chosen.power_mw);
            // An error here is a *result*, not a fault: it means the SoC floor
            // stopped the shave, which is exactly what this pass exists to
            // surface rather than hide behind a silent zero.
            if let Ok(out) = battery.discharge(want, INTERVAL_H) {
                delivered += out;
                if i == peak_index {
                    shaved_at_peak = out / INTERVAL_H;
                }
            }
        } else {
            let want = (ceiling - load).min(chosen.power_mw);
            if want > 0.0 {
                // An error here just means the pack filled up, which is the
                // desired behaviour once the load is under the ceiling.
                let _ = battery.charge(want, INTERVAL_H);
            }
        }
        min_soc = min_soc.min(battery.soc);
    }

    let cycles = battery.cumulative_throughput_mwh / (chosen.energy_mwh * 0.8);
    let model = DegradationModel::li_ion();
    // Isolate the cycle-driven term: calendar aging applies over the project
    // life regardless of duty and is not what this sizing turns on.
    let cycle_fade = model.calculate_degradation(cycles, 0.0, model.reference_temperature_c);

    Validated {
        usable_hours: (1.0 - MIN_SOC) * chosen.energy_mwh / chosen.power_mw.max(1e-9),
        min_soc,
        cycles,
        cycle_fade_percent: cycle_fade * 100.0,
        delivered_mwh: delivered,
        shaved_peak_mw: shaved_at_peak,
    }
}
