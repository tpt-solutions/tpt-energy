//! `tpt-nrg run` — power-flow solving and reporting.

use tpt_nrg_core::EnergySystem;
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowResult, PowerFlowSolver};

use crate::{CliError, MethodArg, OutputFormat, RunArgs};

/// Solve `system` with `method`, or return `None` when it does not converge.
///
/// The default tolerances come from the command line so that a caller of
/// [`viz_command`](crate::viz_command) can reuse the same solver settings.
pub fn solve(system: &EnergySystem, method: PowerFlowMethod) -> Result<PowerFlowResult, CliError> {
    PowerFlowSolver::new(method)
        .solve(system)
        .map_err(|e| CliError::Analysis(e.to_string()))
}

/// Run a power flow and print the solution.
pub fn run(args: &RunArgs, system: &EnergySystem, format: OutputFormat) -> Result<(), CliError> {
    let method: PowerFlowMethod = args.method.into();
    let solver = PowerFlowSolver::new(method)
        .with_tolerance(args.tolerance)
        .with_max_iterations(args.max_iterations);
    let result = solver
        .solve(system)
        .map_err(|e| CliError::Analysis(e.to_string()))?;

    match format {
        OutputFormat::Json => print_json(system, &result),
        OutputFormat::Table => print_table(system, &result, args.method),
    }
    if !result.converged {
        return Err(CliError::Analysis(
            "power flow did not converge".to_string(),
        ));
    }
    Ok(())
}

/// Print the solution as JSON.
fn print_json(system: &EnergySystem, result: &PowerFlowResult) {
    let payload = serde_json::json!({
        "system": system.id,
        "converged": result.converged,
        "iterations": result.iterations,
        "final_mismatch": result.final_mismatch,
        "total_losses_mw": result.total_losses_mw,
        "total_losses_mvar": result.total_losses_mvar,
        "buses": system
            .buses
            .iter()
            .enumerate()
            .map(|(i, b)| serde_json::json!({
                "id": b.id,
                "type": b.bus_type,
                "voltage_magnitude_pu": result.bus_voltage_magnitude_pu[i],
                "voltage_angle_deg": result.bus_voltage_angle_rad[i].to_degrees(),
                "generation_mw": result.generator_p_mw.get(i).copied().unwrap_or(0.0),
            }))
            .collect::<Vec<_>>(),
        "branches": system
            .branches
            .iter()
            .zip(result.branch_flows.iter())
            .map(|(b, f)| serde_json::json!({
                "id": b.id,
                "name": b.name,
                "p_from_mw": f.p_from_mw,
                "q_from_mvar": f.q_from_mvar,
                "loading_fraction": f.loading_fraction,
                "overloaded": f.is_overloaded(),
            }))
            .collect::<Vec<_>>(),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    );
}

/// Print the solution as an aligned table.
fn print_table(system: &EnergySystem, result: &PowerFlowResult, method: MethodArg) {
    println!("System      : {} ({})", system.name, system.id);
    println!(
        "Method      : {}  -> converged={} after {} iterations (mismatch {:.3e})",
        method_name(method),
        result.converged,
        result.iterations,
        result.final_mismatch
    );
    println!(
        "Losses      : {:.3} MW, {:.3} MVAr",
        result.total_losses_mw, result.total_losses_mvar
    );
    println!();
    println!(
        "{:>6}  {:<8}  {:>9}  {:>10}  {:>12}",
        "BUS", "TYPE", "|V| pu", "angle deg", "GEN MW"
    );
    println!("{}", "-".repeat(52));
    for (i, bus) in system.buses.iter().enumerate() {
        let v = result
            .bus_voltage_magnitude_pu
            .get(i)
            .copied()
            .unwrap_or(f64::NAN);
        let a = result
            .bus_voltage_angle_rad
            .get(i)
            .copied()
            .unwrap_or(0.0)
            .to_degrees();
        let g = result.generator_p_mw.get(i).copied().unwrap_or(0.0);
        println!(
            "{:>6}  {:<8}  {:>9.4}  {:>10.3}  {:>12.2}",
            bus.id,
            bus_type_name(bus),
            v,
            a,
            g
        );
    }
    println!();
    println!(
        "{:>6}  {:<14}  {:>10}  {:>11}  {:>9}",
        "BRANCH", "NAME", "P MW", "Q MVAr", "LOAD %"
    );
    println!("{}", "-".repeat(58));
    for (br, flow) in system.branches.iter().zip(result.branch_flows.iter()) {
        println!(
            "{:>6}  {:<14}  {:>10.3}  {:>11.3}  {:>8.1}%{}",
            br.id,
            br.name,
            flow.p_from_mw,
            flow.q_from_mvar,
            flow.loading_fraction * 100.0,
            if flow.is_overloaded() {
                "  OVERLOAD"
            } else {
                ""
            }
        );
    }
}

/// Human-readable name of a solver.
fn method_name(method: MethodArg) -> &'static str {
    match method {
        MethodArg::NewtonRaphson => "newton-raphson",
        MethodArg::GaussSeidel => "gauss-seidel",
        MethodArg::FastDecoupled => "fast-decoupled",
        MethodArg::Dc => "dc",
    }
}

/// Short name of a bus type.
fn bus_type_name(bus: &tpt_nrg_core::Bus) -> &'static str {
    use tpt_nrg_core::BusType;
    match bus.bus_type {
        BusType::Slack => "slack",
        BusType::Pv => "pv",
        BusType::Pq => "pq",
        BusType::Isolated => "isolated",
    }
}
