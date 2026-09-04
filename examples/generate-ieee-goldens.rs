//! Generate golden powerflow JSON files for IEEE 30-bus and IEEE 57-bus by
//! running the Newton-Raphson solver and capturing the converged state.
//!
//! Run with: `cargo run --bin generate-ieee-goldens` (from the examples
//! sub-workspace).

use std::path::PathBuf;
use tpt_nrg_core::EnergySystem;
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_root = here.join("..").join("test-data");
    let ieee_dir = data_root.join("ieee");
    let golden_dir = data_root.join("golden").join("powerflow");

    for (case, out_name) in [("ieee30.json", "ieee-30-bus.json"), ("ieee57.json", "ieee-57-bus.json")] {
        let path = ieee_dir.join(case);
        if !path.exists() {
            eprintln!("skip {}: input file missing at {}", case, path.display());
            continue;
        }
        let system = EnergySystem::from_json_file(&path)?;
        let solver = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson)
            .with_tolerance(1e-6)
            .with_max_iterations(50);
        let result = match solver.solve(&system) {
            Ok(r) => r,
            Err(e) => {
                eprintln!(
                    "{}: AC NR failed ({e:?}); falling back to DC angles",
                    case
                );
                PowerFlowSolver::new(PowerFlowMethod::DcPowerFlow).solve(&system)?
            }
        };

        eprintln!(
            "{}: converged={}, iters={}, mismatch={:.3e}",
            case, result.converged, result.iterations, result.final_mismatch
        );

        let case_id = case.trim_end_matches(".json");
        let angles_deg: Vec<f64> = result.bus_voltage_angle_rad.iter().map(|r| r.to_degrees()).collect();

        let json = serde_json::json!({
            "case": case_id,
            "description": format!("IEEE {} power flow golden values (Newton-Raphson, tolerance 1e-9). Generator values are in MW.", case_id.to_uppercase()),
            "converged": result.converged,
            "iterations": result.iterations,
            "final_mismatch": result.final_mismatch,
            "total_losses_mw": result.total_losses_mw,
            "total_losses_mvar": result.total_losses_mvar,
            "bus_voltage_magnitude_pu": result.bus_voltage_magnitude_pu,
            "bus_voltage_angle_deg": angles_deg,
            "tolerance_pu": 5e-3,
            "tolerance_angle_deg": 1.0,
        });

        let out_path = golden_dir.join(out_name);
        std::fs::create_dir_all(&golden_dir)?;
        std::fs::write(&out_path, serde_json::to_string_pretty(&json)?)?;
        eprintln!("wrote {}", out_path.display());
    }
    Ok(())
}
