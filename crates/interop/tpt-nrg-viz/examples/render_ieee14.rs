//! Render a committed IEEE test case to `target/ieee14.svg`.
//!
//! Run with: `cargo run -p tpt-nrg-viz --example render_ieee14`

use tpt_nrg_core::EnergySystem;
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};
use tpt_nrg_viz::{render, VizOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let system = EnergySystem::from_json(include_str!("../../../../test-data/ieee/ieee14.json"))?;
    let result = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson).solve(&system)?;
    let svg = render(&system, Some(&result), &VizOptions::default());

    let out = std::path::Path::new("target").join("ieee14.svg");
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out, &svg)?;
    println!(
        "wrote {} ({} bytes); converged={} in {} iterations, losses {:.3} MW",
        out.display(),
        svg.len(),
        result.converged,
        result.iterations,
        result.total_losses_mw
    );
    Ok(())
}
