//! Solve the IEEE 14-bus power flow and print the results.
//!
//! Run with: `cargo run --example ieee-14-bus-powerflow`

use tpt_nrg_core::EnergySystem;
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("test-data")
        .join("ieee")
        .join("ieee14.json");
    let system = EnergySystem::from_json_file(&path)?;
    println!(
        "Loaded {} ({} buses, {} branches, {} generators, {} loads)",
        system.name,
        system.buses.len(),
        system.branches.len(),
        system.generators.len(),
        system.loads.len()
    );

    let solver = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson)
        .with_tolerance(1e-6)
        .with_max_iterations(50);
    let result = solver.solve(&system)?;

    println!("\nConverged: {} in {} iterations (mismatch {:.3e})",
        result.converged, result.iterations, result.final_mismatch);
    println!("\nBus voltages:");
    for (i, bus) in system.buses.iter().enumerate() {
        println!(
            "  Bus {:>2} |V|={:.4} pu,  θ={:+7.3}°",
            bus.id,
            result.bus_voltage_magnitude_pu[i],
            result.bus_voltage_angle_rad[i].to_degrees()
        );
    }
    println!("\nTotal losses: {:.3} MW, {:.3} MVAr",
        result.total_losses_mw, result.total_losses_mvar);
    Ok(())
}
