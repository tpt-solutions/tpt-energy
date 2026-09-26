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

/// IEEE 57-bus topology sanity: dimensions and bus-type census.
#[test]
fn ieee57_topology_loads() {
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
}

/// Phase 2 milestone: IEEE 57-bus AC power flow converges under
/// Newton–Raphson with Q-limit enforcement, matches the golden (AC) values,
/// and reproduces the published MATPOWER case57 solution anchors to well
/// within 1%: total losses ≈ 27.86 MW, slack dispatch ≈ 478.66 MW, and
/// slack reactive output ≈ 128.85 `MVAr`.
#[test]
fn ieee57_ac_matches_golden_and_published() {
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

    let solver = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson)
        .with_max_iterations(200)
        .with_tolerance(1e-8);
    let r = solver.solve(&system).expect("ieee57 AC solve");
    assert!(
        r.converged,
        "ieee57 AC must converge (mismatch {:.3e})",
        r.final_mismatch
    );

    // Compare against the AC golden fixture (regenerated together with the
    // solver by `generate-ieee-goldens`).
    let golden_raw = std::fs::read_to_string(golden_path().join("ieee-57-bus.json"))
        .expect("read golden ieee-57");
    let golden: Golden = serde_json::from_str(&golden_raw).expect("parse golden");
    let deg_tol = if golden.tolerance_angle_deg > 0.0 {
        golden.tolerance_angle_deg
    } else {
        1.0
    };
    for (i, (&got, &want)) in r
        .bus_voltage_magnitude_pu
        .iter()
        .zip(golden.bus_voltage_magnitude_pu.iter())
        .enumerate()
    {
        assert!(
            (got - want).abs() < 1e-6,
            "bus {i} V: got {got}, want {want}"
        );
    }
    for (i, (&got, &want)) in r
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

    // Published anchors (MATPOWER case57 reference solution).
    let anchor = |got: f64, published: f64, what: &str| {
        let err = ((got - published) / published).abs();
        assert!(
            err < 0.01,
            "{what}: got {got:.3}, published {published:.3} (rel err {err:.3e} > 1%)"
        );
    };
    anchor(r.total_losses_mw, 27.86, "total losses");
    anchor(r.generator_p_mw[0], 478.66, "slack P");
    anchor(r.generator_q_mvar[0], 128.85, "slack Q");
}
