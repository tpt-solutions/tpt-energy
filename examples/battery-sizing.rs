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
//! 3. **Size** the battery for a target demand reduction, by simulating the
//!    charge/discharge the profile actually demands rather than using the
//!    rule of thumb "peak x hours".
//! 4. **Validate** the size with a real [`tpt_nrg_battery::BatteryStorage`]
//!    SoC model, including round-trip efficiency and the minimum-SoC floor.
//! 5. **Cost it out** with [`tpt_nrg_lcoe`], and report the payback against
//!    avoided demand charges.
//!
//! Step 3 is a *simulation* over the real profile on purpose. The tempting
//! shortcut — "energy = reduction x window hours" — ignores that the battery
//! cannot be full and empty at the same time, cannot discharge below its
//! floor, and loses energy to round-trip losses. Those three effects decide
//! whether a quoted size actually works, so the size is found by simulating
//! them rather than by arithmetic.

use std::path::{Path, PathBuf};

use tpt_nrg_battery::{BatteryStorage, DegradationModel};
use tpt_nrg_lcoe::{levelized_cost_of_energy, LcoeInputs};

/// Target demand reduction, in MW.
const TARGET_MW: f64 = 8.0;
/// Round-trip efficiency of the assumed chemistry.
const ROUND_TRIP: f64 = 0.90;
/// Battery capital cost, in dollars per kWh of nameplate capacity.
const CAPEX_PER_KWH: f64 = 250.0;
/// Annual fixed O&M, as a fraction of CAPEX.
const FIXED_OM_FRACTION: f64 = 0.015;
/// Project lifetime, in years.
const LIFETIME_YEARS: u32 = 15;
/// Discount rate.
const DISCOUNT_RATE: f64 = 0.07;
/// Days assumed per year when scaling the profile up.
const DAYS_PER_YEAR: f64 = 365.0;
/// Cost of one MW of peak, in dollars per year.
const PEAK_CHARGE_PER_MW_YEAR: f64 = 18_000.0;
/// Spacing of the committed fixtures, in hours.
const INTERVAL_H: f64 = 0.25;

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
    println!("  energy over the window: {:.1} MWh", profile.iter().sum::<f64>());

    // ----------------------------------------------------------------
    // 2. Characterise
    // ----------------------------------------------------------------
    let peak = profile.iter().copied().fold(0.0_f64, f64::max);
    let mean = profile.iter().sum::<f64>() / profile.len() as f64;
    let peak_index = profile
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(i, _)| i)
        .unwrap_or(0);
    println!("\nStep 2: Characterisation");
    println!(
        "  peak        : {peak:.2} MW at hour {:.2}",
        peak_index as f64 * INTERVAL_H
    );
    println!("  mean        : {mean:.2} MW");
    println!("  load factor : {:.2}", mean / peak);

    // ----------------------------------------------------------------
    // 3. Size by simulation
    // ----------------------------------------------------------------
    println!("\nStep 3: Sizing sweep (target {TARGET_MW:.1} MW reduction)");
    println!(
        "  {:>10}  {:>10}  {:>12}  {:>13}  {}",
        "power MW", "energy MWh", "shaved MW", "cycles/day", "meets target"
    );
    println!("  {}", "-".repeat(66));

    let sweep = sweep(&profile, peak_index);
    for c in &sweep.candidates {
        println!(
            "  {:>10.1}  {:>10.1}  {:>12.2}  {:>13.2}  {}",
            c.power_mw,
            c.energy_mwh,
            c.shaved_peak_mw,
            c.cycles_per_day,
            if c.meets_target { "yes" } else { "no" }
        );
    }
    let chosen = sweep.chosen.clone();
    println!(
        "\n  smallest size that holds the target: {:.1} MW / {:.2} MWh",
        chosen.power_mw, chosen.energy_mwh
    );

    // ----------------------------------------------------------------
    // 4. Validate with the real SoC model
    // ----------------------------------------------------------------
    println!("\nStep 4: Validation with the SoC model");
    let v = validate(&profile, &chosen);
    println!("  power rating     : {:.1} MW", chosen.power_mw);
    println!("  nameplate energy : {:.2} MWh", chosen.energy_mwh);
    println!("  usable window    : {:.2} h at full power", v.usable_hours);
    println!(
        "  min SoC reached  : {:.1}% (floor {:.0}%)",
        v.min_soc * 100.0,
        MIN_SOC * 100.0
    );
    println!(
        "  shaved at peak   : {:.2} MW (target {TARGET_MW:.1} MW)",
        v.shaved_peak_mw
    );
    println!(
        "  energy delivered : {:.2} MWh over {hours:.2} h",
        v.delivered_mwh, hours
    );
    println!("  equivalent cycles: {:.2}", v.cycles);
    println!(
        "  capacity fade    : {:.1}% over {LIFETIME_YEARS} y at this duty cycle",
        v.fade_percent
    );
    if v.shortfall_mw > 1e-6 {
        println!(
            "  NOTE: the SoC floor stopped the shave {:.2} MW short at the peak;\n  the sweep sized on an approximation, so size up before buying.",
            v.shortfall_mw
        );
    } else {
        println!("  the SoC model holds the target through the peak window");
    }

    // ----------------------------------------------------------------
    // 5. Cost it out
    // ----------------------------------------------------------------
    let capex = chosen.energy_mwh * 1000.0 * CAPEX_PER_KWH;
    let annual_energy = profile.iter().sum::<f64>() * DAYS_PER_YEAR;
    let lcoe = levelized_cost_of_energy(&LcoeInputs::new(
        capex,
        capex * FIXED_OM_FRACTION,
        0.0,
        0.0,
        annual_energy,
        LIFETIME_YEARS,
        DISCOUNT_RATE,
        0.0,
    ));
    let payback = capex / (chosen.power_mw * PEAK_CHARGE_PER_MW_YEAR);

    println!("\nStep 5: Cost");
    println!("  CAPEX                : ${capex:.0} (${CAPEX_PER_KWH:.0}/kWh)");
    println!("  annual fixed O&M     : ${:.0}", capex * FIXED_OM_FRACTION);
    println!("  annual energy served : {annual_energy:.0} MWh");
    println!(
        "  battery LCOE         : ${lcoe:.2}/MWh (lifetime {LIFETIME_YEARS} y, discount {DISCOUNT_RATE:.0%})"
    );
    println!("  cycles per day       : {:.2}", v.cycles / hours);
    println!(
        "\n  Against a ${PEAK_CHARGE_PER_MW_YEAR:.0}/MW-year demand charge, this pays\n\
         back in about {payback:.1} years of demand charges alone."
    );

    Ok(())
}

/// A candidate battery size and how it performed on the profile.
#[derive(Debug, Clone)]
struct Candidate {
    /// Power rating, in MW.
    power_mw: f64,
    /// Nameplate energy, in MWh.
    energy_mwh: f64,
    /// Reduction at the system peak, in MW.
    shaved_peak_mw: f64,
    /// Full charge/discharge cycles per day.
    cycles_per_day: f64,
    /// Whether it met the target reduction.
    meets_target: bool,
}

/// Every candidate in the sweep, plus the chosen one.
struct Sweep {
    /// All candidates, cheapest first.
    candidates: Vec<Candidate>,
    /// The smallest that met the target.
    chosen: Candidate,
}

/// What the SoC model says about a chosen size.
struct Validated {
    /// Hours the battery can hold full power from full to the floor.
    usable_hours: f64,
    /// Lowest state of charge reached, as a fraction.
    min_soc: f64,
    /// Equivalent full cycles accumulated over the profile.
    cycles: f64,
    /// Capacity fade over the assumed project life, in percent.
    fade_percent: f64,
    /// Energy the battery actually delivered, in MWh.
    delivered_mwh: f64,
    /// Reduction at the system peak, in MW, as delivered.
    shaved_peak_mw: f64,
    /// How far the SoC model fell short of the target at the peak, in MW.
    shortfall_mw: f64,
}

/// Read a `timestamp_utc,load_mw` CSV into a load profile in MW.
///
/// The timestamp column is read so the schema is validated, but the sizing
/// only needs the MW series.
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

/// The battery will not discharge below this state of charge.
const MIN_SOC: f64 = 0.10;
/// Margin applied to the energy the duty cycle actually needs.
const SIZE_MARGIN: f64 = 1.25;
/// Shortest useful discharge window, in hours.
const MIN_WINDOW_H: f64 = 0.5;

/// Sweep candidate power ratings, sizing the energy from the duty cycle.
///
/// For each candidate the battery is dispatched greedily — discharge whenever
/// the load is above the ceiling, charge from whatever headroom is left — and
/// the *energy* is then derived from how far the SoC actually swung. The
/// energy is therefore an output of the simulation rather than an input,
/// which is what makes the resulting size defensible: it already accounts for
/// the fact that the battery must refill before it can shave again.
fn sweep(profile: &[f64], peak_index: usize) -> Sweep {
    let peak = profile[peak_index];
    let ceiling = peak - TARGET_MW;
    let mut candidates = Vec::new();

    for power_mw in [2.0_f64, 4.0, 6.0, 8.0, 10.0] {
        // Dispatch on a unit-energy battery: only the SoC *swing* matters,
        // and it tells us how much nameplate the duty cycle needs.
        let unit = UnitBattery::new(power_mw, MIN_SOC);
        let mut soc = 1.0_f64;
        let mut min_soc = 1.0_f64;
        let mut max_soc = 0.0_f64;
        // Only the peak reduction is needed to judge sufficiency.
        let mut delivered_at_peak = 0.0_f64;
        let mut full_cycles = 0.0_f64;
        let mut prev_soc = soc;
        for (i, &load) in profile.iter().enumerate() {
            let power = unit.dispatch(soc, load, ceiling);
            soc = (soc - power / ASSUMED_ENERGY_MWH).clamp(MIN_SOC, 1.0);
            max_soc = max_soc.max(soc);
            min_soc = min_soc.min(soc);
            // Accumulate the SoC swing as equivalent full cycles.
            full_cycles += (prev_soc - soc).abs();
            prev_soc = soc;
            if i == peak_index {
                delivered_at_peak = power.max(0.0);
            }
        }

        // Nameplate: the usable swing the profile demanded, grossed up for the
        // SoC floor, the losses, and a refill margin.
        let usable_swing = (max_soc - min_soc).max(0.0) * ASSUMED_ENERGY_MWH;
        let nameplate = usable_swing / (1.0 - MIN_SOC) / ROUND_TRIP * SIZE_MARGIN;
        let energy_mwh = nameplate.max(power_mw * MIN_WINDOW_H);
        let meets = delivered_at_peak >= TARGET_MW - 1e-6;

        candidates.push(Candidate {
            power_mw,
            energy_mwh,
            shaved_peak_mw: delivered_at_peak,
            cycles_per_day: full_cycles,
            meets_target: meets,
        });
    }

    // Capital cost tracks kWh, so the cheapest sufficient size is the one with
    // the least energy among those that actually hold the target.
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

    Sweep {
        candidates,
        chosen,
    }
}

/// Nominal energy used to normalise the duty-cycle simulation, in MWh.
///
/// The dispatch ratios are scale-invariant, so any positive value yields the
/// same relative behaviour; the real nameplate is derived from the swing.
const ASSUMED_ENERGY_MWH: f64 = 1.0;

/// A power-limited battery on unit energy, for the sizing dispatch.
struct UnitBattery {
    /// Power rating, in MW.
    power_mw: f64,
    /// Minimum state of charge, as a fraction.
    min_soc: f64,
}

impl UnitBattery {
    /// Construct a unit-energy battery with the given rating and floor.
    fn new(power_mw: f64, min_soc: f64) -> Self {
        Self { power_mw, min_soc }
    }

    /// Greedy power for this interval, in MW, positive when discharging.
    ///
    /// Discharge is preferred over charging: a battery that charges into a
    /// peak it could have shaved is worse than useless.
    fn dispatch(&self, soc: f64, load: f64, ceiling: f64) -> f64 {
        if load > ceiling && soc > self.min_soc {
            // Available stored energy over this interval, grossed up for the
            // discharge leg of the round trip.
            let available = (soc - self.min_soc) * ASSUMED_ENERGY_MWH * ROUND_TRIP.sqrt();
            (load - ceiling).min(self.power_mw).min(available / INTERVAL_H).max(0.0)
        } else if load < ceiling {
            let room = (1.0 - soc) * ASSUMED_ENERGY_MWH;
            let want = (ceiling - load).min(self.power_mw);
            (want * ROUND_TRIP.sqrt()).min(room / INTERVAL_H).max(0.0) * -1.0
        } else {
            0.0
        }
    }
}

/// Replay the chosen size through the real [`BatteryStorage`] model.
///
/// The sizing sweep is a fast approximation. This pass uses the crate's own
/// SoC model so that round-trip efficiency, the power rating, the SoC floor,
/// and cumulative throughput are all *enforced* rather than assumed. If the
/// two disagree, this is the number to believe.
fn validate(profile: &[f64], chosen: &Candidate) -> Validated {
    let peak = profile.iter().copied().fold(0.0_f64, f64::max);
    let ceiling = peak - TARGET_MW;

    let mut battery = BatteryStorage::new(chosen.energy_mwh, chosen.power_mw, ROUND_TRIP)
        .with_soc(MIN_SOC, 1.0)
        .with_degradation(DegradationModel::li_ion());
    let mut min_soc = battery.soc;
    let mut delivered = 0.0_f64;
    let mut delivered_at_peak = 0.0_f64;
    let mut shortfall = 0.0_f64;

    for (i, &load) in profile.iter().enumerate() {
        if load > ceiling {
            let want = (load - ceiling).min(chosen.power_mw);
            // A refusal here is meaningful, not exceptional: it means the
            // SoC floor stopped the shave, which is exactly what we want to
            // surface rather than hide behind a zero.
            match battery.discharge(want, INTERVAL_H) {
                Ok(out) => {
                    let power = out / INTERVAL_H;
                    delivered += out;
                    if i == peak_index {
                        delivered_at_peak = power;
                    }
                }
                Err(_) => {
                    if i == peak_index {
                        shortfall = want - 0.0;
                    }
                }
            }
        } else {
            let want = (ceiling - load).min(chosen.power_mw);
            if want > 0.0 {
                // Ignore the SoC-above-max error: it just means the battery
                // filled up, which is the desired behaviour at the ceiling.
                let _ = battery.charge(want, INTERVAL_H);
            }
        }
        min_soc = min_soc.min(battery.soc);
    }

    // Degradation at the observed duty, over the assumed project life.
    let cycles = battery.cumulative_throughput_mwh / (chosen.energy_mwh * 0.8);
    let fade = battery
        .degradation
        .as_ref()
        .map_or(0.0, |m| {
            m.calculate_degradation(
                cycles,
                LIFETIME_YEARS as f64,
                m.reference_temperature_c,
            )
        });

    Validated {
        usable_hours: (1.0 - MIN_SOC) * chosen.energy_mwh / chosen.power_mw.max(1e-9),
        min_soc,
        cycles,
        fade_percent: fade * 100.0,
        delivered_mwh: delivered,
        shaved_peak_mw: delivered_at_peak,
        shortfall_mw: shortfall,
    }
}
