//! `{{project-name}}` — an energy-system study built with TPT Energy.
//!
//! Loads `system.json`, solves a Newton-Raphson power flow, and prints the
//! per-bus voltage profile and total losses.
//!
//! Replace `system.json` with your own case (MATPOWER, PSS/E, CIM, YAML, and
//! CSV can all be converted to the native JSON with the `tpt-nrg` CLI), or
//! build the `EnergySystem` in code with the `tpt-nrg-core` constructors.

use tpt_nrg_core::EnergySystem;
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};

/// The case to study. Change this path to point at your own file.
const CASE: &str = include_str!("../system.json");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let system = EnergySystem::from_json(CASE)?;
    println!("case {}: {} buses", system.name, system.buses.len());

    let solver = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson);
    let result = solver.solve(&system)?;

    println!("converged in {} iterations", result.iterations);
    println!("total losses: {:.3} MW", result.total_losses_mw);
    for (index, voltage) in result.bus_voltage_magnitude_pu.iter().enumerate() {
        let id = system.buses.get(index).map_or(index + 1, |bus| bus.id);
        println!("  bus {id:>3}: |V| = {voltage:.4} pu");
    }

    Ok(())
}
