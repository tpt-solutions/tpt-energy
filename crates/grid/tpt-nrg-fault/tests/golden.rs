//! Golden-value validation for fault analysis at IEEE 14-bus case 4.

use std::path::PathBuf;
use tpt_nrg_core::EnergySystem;
use tpt_nrg_fault::{FaultAnalyzer, FaultType};

#[derive(serde::Deserialize)]
struct FaultCase {
    fault_bus: usize,
    faults: Faults,
}

#[derive(serde::Deserialize)]
struct Faults {
    three_phase: SingleFault,
    line_to_line: SingleFault,
    line_to_ground: SingleFault,
    double_line_to_ground: SingleFault,
}

#[derive(serde::Deserialize)]
struct SingleFault {
    i_fault_pu: f64,
    tolerance_pu: f64,
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("test-data")
        .join("golden")
        .join("fault")
}

fn ieee14_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("test-data")
        .join("ieee")
        .join("ieee14.json")
}

#[test]
fn ieee14_bus4_faults_match_golden() {
    let golden_raw = std::fs::read_to_string(golden_path().join("ieee14-bus4.json"))
        .expect("read golden fault file");
    let golden: FaultCase = serde_json::from_str(&golden_raw).expect("parse golden");
    let system = EnergySystem::from_json_file(ieee14_path()).expect("load ieee14");
    let a = FaultAnalyzer::new(&system);

    let check = |label: &str, ft: FaultType, want: f64, tol: f64| {
        let r = a.calculate_fault_current(golden.fault_bus, ft);
        assert!(
            (r.i_fault_pu - want).abs() < tol,
            "{label}: got {:.3} pu, want {want} ± {tol}",
            r.i_fault_pu
        );
    };
    check(
        "3φ",
        FaultType::ThreePhase,
        golden.faults.three_phase.i_fault_pu,
        golden.faults.three_phase.tolerance_pu,
    );
    check(
        "L-L",
        FaultType::LineToLine,
        golden.faults.line_to_line.i_fault_pu,
        golden.faults.line_to_line.tolerance_pu,
    );
    check(
        "SLG",
        FaultType::LineToGround,
        golden.faults.line_to_ground.i_fault_pu,
        golden.faults.line_to_ground.tolerance_pu,
    );
    check(
        "DLG",
        FaultType::DoubleLineToGround,
        golden.faults.double_line_to_ground.i_fault_pu,
        golden.faults.double_line_to_ground.tolerance_pu,
    );
}
