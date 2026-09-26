//! `tpt-nrg run --fault-bus` — three-phase short-circuit study.

use tpt_nrg_core::EnergySystem;
use tpt_nrg_fault::{FaultAnalyzer, FaultType};

use crate::{CliError, OutputFormat};

/// Compute and print the fault current at `bus` for every fault type.
pub fn run(system: &EnergySystem, bus: usize, format: OutputFormat) -> Result<(), CliError> {
    if !system.buses.iter().any(|b| b.id == bus) {
        return Err(CliError::Usage(format!("no bus with id {bus}")));
    }
    let analyzer = FaultAnalyzer::new(system);
    let types = [
        FaultType::ThreePhase,
        FaultType::LineToLine,
        FaultType::LineToGround,
        FaultType::DoubleLineToGround,
    ];
    let results: Vec<_> = types
        .iter()
        .map(|t| analyzer.calculate_fault_current(bus, *t))
        .collect();

    match format {
        OutputFormat::Json => {
            let payload = serde_json::json!({
                "system": system.id,
                "bus": bus,
                "base_mva": system.base_mva,
                "faults": results
                    .iter()
                    .map(|r| serde_json::json!({
                        "type": r.fault_type,
                        "i_fault_pu": r.i_fault_pu,
                        "i_fault_amps": r.i_fault_amps,
                        "z1_pu": r.z1_pu,
                        "z0_pu": r.z0_pu,
                    }))
                    .collect::<Vec<_>>(),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_default()
            );
        }
        OutputFormat::Table => {
            println!("System   : {} ({})", system.name, system.id);
            println!("Fault bus: {bus}   (base {:.1} MVA)", system.base_mva);
            println!();
            println!(
                "{:>24}  {:>12}  {:>14}  {:>10}",
                "FAULT TYPE", "I pu", "I (A)", "Z1 pu"
            );
            println!("{}", "-".repeat(66));
            for r in &results {
                println!(
                    "{:>24}  {:>12.4}  {:>14.1}  {:>10.4}",
                    fault_name(r.fault_type),
                    r.i_fault_pu,
                    r.i_fault_amps,
                    r.z1_pu
                );
            }
        }
    }
    Ok(())
}

/// Human-readable name of a fault type.
fn fault_name(t: FaultType) -> &'static str {
    match t {
        FaultType::ThreePhase => "three-phase",
        FaultType::LineToLine => "line-to-line",
        FaultType::LineToGround => "line-to-ground",
        FaultType::DoubleLineToGround => "double-line-to-ground",
    }
}
