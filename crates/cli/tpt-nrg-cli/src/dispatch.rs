//! `tpt-nrg run` — economic dispatch, unit commitment, LCOE, and carbon.

use tpt_nrg_core::EnergySystem;
use tpt_nrg_economic_dispatch::economic_dispatch;
use tpt_nrg_lcoe::{levelized_cost_of_energy, LcoeInputs};
use tpt_nrg_unit_commitment::unit_commitment;

use crate::{CliError, OutputFormat, RunArgs};

/// Run a lossless economic dispatch at the requested load and report it.
pub fn run_economic(
    system: &EnergySystem,
    load_mw: f64,
    format: OutputFormat,
) -> Result<(), CliError> {
    require_dispatchable(system)?;
    let result = economic_dispatch(system, load_mw)
        .map_err(|e| CliError::Analysis(format!("economic dispatch: {e}")))?;
    match format {
        OutputFormat::Json => print_economic_json(system, load_mw, &result),
        OutputFormat::Table => print_economic_table(system, load_mw, &result),
    }
    Ok(())
}

/// Run a priority-list unit commitment over a 24-hour profile derived from the
/// system's own load, and report the schedule.
pub fn run_commitment(system: &EnergySystem, format: OutputFormat) {
    let profile = vec![system.total_load_mw(); 24];
    let result = unit_commitment(system, &profile);
    // The commitment order comes from each unit's average cost, and the cost
    // comes from its curve; a case without curves commits on capacity alone and
    // prices everything at zero, so say so rather than printing a bare $0.00.
    let priced = system.generators.iter().any(|g| g.cost_curve.is_some());
    match format {
        OutputFormat::Json => {
            let payload = serde_json::json!({
                "intervals": profile.len(),
                "commitment": result.commitment,
                "outputs_mw": result.outputs,
                "total_cost_dollar": result.total_cost_dollar,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_default()
            );
        }
        OutputFormat::Table => {
            println!("24-hour priority-list commitment at {:.2} MW:", profile[0]);
            println!("{:>6}  {:>12}  {:>14}", "UNIT", "COMMITTED h", "ENERGY MWh");
            println!("{}", "-".repeat(36));
            for (i, gen) in system.generators.iter().enumerate() {
                let hours: i64 = result.commitment[i].iter().map(|&c| i64::from(c)).sum();
                let energy: f64 = result.outputs[i].iter().sum();
                println!("{:>6}  {:>12}  {:>14.2}", gen.id, hours, energy);
            }
            println!();
            println!("Total cost   : ${:.2}", result.total_cost_dollar);
            if !priced {
                println!(
                    "Note: no generator has a cost curve, so the merit order falls back to \
                     declared cost order and every unit prices at $0."
                );
            }
        }
    }
}

/// Compute the levelized cost of energy from the `--lcoe-*` flags.
pub fn run_lcoe(
    args: &RunArgs,
    system: &EnergySystem,
    format: OutputFormat,
) -> Result<(), CliError> {
    let capex = args.lcoe_capex.unwrap_or(0.0);
    let energy = args.annual_energy_mwh.unwrap_or(0.0);
    if energy <= 0.0 {
        return Err(CliError::Usage(
            "--annual-energy-mwh must be greater than zero".to_string(),
        ));
    }
    let inputs = LcoeInputs {
        capex_dollar: capex,
        annual_fixed_om_dollar: args.lcoe_fixed_om,
        variable_om_dollar_per_mwh: 0.0,
        fuel_cost_dollar_per_mwh: 0.0,
        annual_energy_mwh: energy,
        lifetime_years: args.lifetime_years,
        discount_rate: args.discount_rate,
        capacity_factor: energy / (system.base_mva * 8760.0).max(1.0),
    };
    let lcoe = levelized_cost_of_energy(&inputs);
    match format {
        OutputFormat::Json => {
            let payload = serde_json::json!({
                "system": system.id,
                "lcoe_dollar_per_mwh": lcoe,
                "capex_dollar": capex,
                "annual_energy_mwh": energy,
                "lifetime_years": args.lifetime_years,
                "discount_rate": args.discount_rate,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_default()
            );
        }
        OutputFormat::Table => {
            println!("System            : {} ({})", system.name, system.id);
            println!("CAPEX             : ${capex:.2}");
            println!("Annual energy     : {energy:.2} MWh");
            println!("Lifetime          : {} years", args.lifetime_years);
            println!("Discount rate     : {:.2}%", args.discount_rate * 100.0);
            println!("LCOE              : ${lcoe:.2}/MWh");
        }
    }
    Ok(())
}

/// Reject a dispatch request the model cannot answer usefully.
///
/// `economic_dispatch` treats a unit without a cost curve as must-run at
/// `p_min`, so a case where *no* unit has a curve has zero dispatchable
/// capacity. Reporting that as "load exceeds capacity" is technically true and
/// practically useless, so it is replaced with the actual cause.
fn require_dispatchable(system: &EnergySystem) -> Result<(), CliError> {
    if system.generators.is_empty() {
        return Err(CliError::Usage(
            "--dispatch needs a system with at least one generator".to_string(),
        ));
    }
    let with_curve = system
        .generators
        .iter()
        .filter(|g| g.cost_curve.is_some())
        .count();
    if with_curve == 0 {
        return Err(CliError::Usage(
            "--dispatch needs cost curves: none of this system's generators has one, \
             so there is no merit order to dispatch along. Add a `cost_curve` to a \
             generator, or use the native JSON/YAML format where cost curves can be \
             written."
                .to_string(),
        ));
    }
    Ok(())
}

/// Print a dispatch result as JSON.
fn print_economic_json(
    system: &EnergySystem,
    load_mw: f64,
    result: &tpt_nrg_economic_dispatch::EconomicDispatchResult,
) {
    let payload = serde_json::json!({
        "system": system.id,
        "load_mw": load_mw,
        "total_cost_dollar_per_h": result.total_cost_dollar_per_h,
        "marginal_cost_dollar_per_mwh": result.marginal_cost_dollar_per_mwh,
        "losses_mw": result.losses_mw,
        "generators": system
            .generators
            .iter()
            .zip(result.generator_outputs_mw.iter())
            .map(|(g, p)| serde_json::json!({ "id": g.id, "name": g.name, "output_mw": p }))
            .collect::<Vec<_>>(),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).unwrap_or_default()
    );
}

/// Print a dispatch result as a table.
fn print_economic_table(
    system: &EnergySystem,
    load_mw: f64,
    result: &tpt_nrg_economic_dispatch::EconomicDispatchResult,
) {
    println!("System        : {} ({})", system.name, system.id);
    println!("Load          : {load_mw:.3} MW");
    println!("Total cost    : ${:.2}/h", result.total_cost_dollar_per_h);
    println!(
        "Marginal cost : ${:.2}/MWh",
        result.marginal_cost_dollar_per_mwh
    );
    println!();
    println!(
        "{:>6}  {:<14}  {:>12}  {:>10}",
        "UNIT", "NAME", "OUTPUT MW", "SHARE %"
    );
    println!("{}", "-".repeat(48));
    for (g, p) in system
        .generators
        .iter()
        .zip(result.generator_outputs_mw.iter())
    {
        let share = if load_mw > 0.0 {
            p / load_mw * 100.0
        } else {
            0.0
        };
        println!("{:>6}  {:<14}  {:>12.3}  {:>9.1}%", g.id, g.name, p, share);
    }
}
