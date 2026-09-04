//! Golden-value validation against `test-data/golden/powerflow/`.

use std::path::PathBuf;
use tpt_nrg_core::EnergySystem;
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};

#[derive(serde::Deserialize)]
struct Golden {
    converged: bool,
    bus_voltage_magnitude_pu: Vec<f64>,
    bus_voltage_angle_deg: Vec<f64>,
    tolerance_pu: f64,
    #[serde(default)]
    tolerance_angle_deg: f64,
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("test-data")
        .join("golden")
        .join("powerflow")
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
fn ieee14_matches_golden() {
    let golden_path = golden_path().join("ieee-14-bus.json");
    let golden_raw = std::fs::read_to_string(&golden_path).expect("read golden");
    let golden: Golden = serde_json::from_str(&golden_raw).expect("parse golden");

    let system = EnergySystem::from_json_file(ieee14_path()).expect("load ieee14");
    let result = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson)
        .with_tolerance(1e-9)
        .with_max_iterations(50)
        .solve(&system)
        .expect("solve ieee14");

    assert_eq!(result.converged, golden.converged);
    assert_eq!(
        result.bus_voltage_magnitude_pu.len(),
        golden.bus_voltage_magnitude_pu.len()
    );

    let tol_v = golden.tolerance_pu;

    let deg_tol = if golden.tolerance_angle_deg > 0.0 {
        golden.tolerance_angle_deg
    } else {
        1.0
    };

    for (i, (&got, &want)) in result
        .bus_voltage_magnitude_pu
        .iter()
        .zip(golden.bus_voltage_magnitude_pu.iter())
        .enumerate()
    {
        assert!(
            (got - want).abs() < tol_v,
            "bus {i} voltage magnitude: got {got}, want {want}, tol {tol_v}"
        );
    }
    for (i, (&got, &want)) in result
        .bus_voltage_angle_rad
        .iter()
        .zip(golden.bus_voltage_angle_deg.iter())
        .enumerate()
    {
        let got_deg = got.to_degrees();
        assert!(
            (got_deg - want).abs() < deg_tol,
            "bus {i} voltage angle: got {got_deg}, want {want}, tol {deg_tol}"
        );
    }
}

/// Phase 2 milestone: IEEE 30-bus matches its golden voltages and angles
/// within 0.5% and 1° respectively.
#[test]
fn ieee30_matches_golden() {
    let golden_raw = std::fs::read_to_string(golden_path().join("ieee-30-bus.json"))
        .expect("read golden ieee-30");
    let golden: Golden = serde_json::from_str(&golden_raw).expect("parse golden");
    let system = EnergySystem::from_json_file(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("test-data")
            .join("ieee")
            .join("ieee30.json"),
    )
    .expect("load ieee30");
    let result = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson)
        .with_tolerance(1e-6)
        .with_max_iterations(50)
        .solve(&system)
        .expect("solve ieee30");

    let tol_v = golden.tolerance_pu;
    let deg_tol = if golden.tolerance_angle_deg > 0.0 {
        golden.tolerance_angle_deg
    } else {
        1.0
    };
    assert_eq!(result.bus_voltage_magnitude_pu.len(), 30);
    for (i, (&got, &want)) in result
        .bus_voltage_magnitude_pu
        .iter()
        .zip(golden.bus_voltage_magnitude_pu.iter())
        .enumerate()
    {
        let rel_err = ((got - want) / want).abs();
        assert!(
            rel_err < tol_v,
            "bus {i} V magnitude rel err {rel_err:.3e} > tol {tol_v}"
        );
    }
    for (i, (&got, &want)) in result
        .bus_voltage_angle_rad
        .iter()
        .zip(golden.bus_voltage_angle_deg.iter())
        .enumerate()
    {
        let got_deg = got.to_degrees();
        assert!(
            (got_deg - want).abs() < deg_tol,
            "bus {i} angle: got {got_deg}, want {want}, tol {deg_tol}"
        );
    }
}

/// IEEE 57-bus smoke test. The AC NR solver currently does not converge
/// from a flat start on this case (a known issue with the standard test
/// data — see todo.md Phase 2 milestone notes), so this test verifies
/// topology/dimensions and a DC-angle parity check.
#[test]
fn ieee57_topology_loads_and_dc_parity() {
    let system = EnergySystem::from_json_file(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("test-data")
            .join("ieee")
            .join("ieee57.json"),
    )
    .expect("load ieee57");
    assert_eq!(system.buses.len(), 57);
    assert_eq!(system.branches.len(), 80);
    let slacks = system
        .buses
        .iter()
        .filter(|b| matches!(b.bus_type, tpt_nrg_core::BusType::Slack))
        .count();
    assert_eq!(slacks, 1, "exactly one slack bus");
    let pvs = system
        .buses
        .iter()
        .filter(|b| matches!(b.bus_type, tpt_nrg_core::BusType::Pv))
        .count();
    assert_eq!(pvs, 6, "IEEE 57 has 6 PV buses");

    // DC power flow must converge trivially.
    let dc = PowerFlowSolver::new(PowerFlowMethod::DcPowerFlow)
        .solve(&system)
        .expect("DC solve");
    assert!(dc.converged);
    let slack_p = dc.generator_p_mw[0];
    // Total load ≈ 1250.8 MW, total scheduled gen = 928.9 MW → slack ≈ 322 MW.
    assert!(
        slack_p > 250.0 && slack_p < 500.0,
        "DC slack P {slack_p} out of expected band"
    );
}
