//! Golden tests for unit commitment and economic dispatch.

use std::path::PathBuf;
use tpt_nrg_core::{Bus, BusType, CostCurve, EnergySystem, Generator, GeneratorType};
use tpt_nrg_economic_dispatch::economic_dispatch;
use tpt_nrg_unit_commitment::unit_commitment;

fn uc_system() -> EnergySystem {
    let mut s = EnergySystem::new("uc", "UC", 100.0, 60.0);
    s.add_bus(Bus::new(1, "B1", BusType::Slack)).unwrap();
    s.add_bus(Bus::new(2, "B2", BusType::Pv)).unwrap();
    s.add_bus(Bus::new(3, "B3", BusType::Pv)).unwrap();
    s.add_generator(
        Generator::new(1, "G1", GeneratorType::Thermal, 100.0, 20.0)
            .at_bus(1)
            .with_cost_curve(CostCurve::piecewise(20.0, 100.0, 20.0, 20.0)),
    )
    .unwrap();
    s.add_generator(
        Generator::new(2, "G2", GeneratorType::Thermal, 80.0, 15.0)
            .at_bus(2)
            .with_cost_curve(CostCurve::piecewise(15.0, 80.0, 30.0, 30.0)),
    )
    .unwrap();
    s.add_generator(
        Generator::new(3, "G3", GeneratorType::Thermal, 50.0, 5.0)
            .at_bus(3)
            .with_cost_curve(CostCurve::piecewise(5.0, 50.0, 50.0, 50.0)),
    )
    .unwrap();
    s
}

fn ed_system() -> EnergySystem {
    let mut sys = EnergySystem::new("ed", "ED", 100.0, 60.0);
    for i in 1..=3 {
        sys.add_bus(Bus::new(i, format!("B{i}"), BusType::Pv)).unwrap();
    }
    sys.add_generator(
        Generator::new(1, "G1", GeneratorType::Thermal, 100.0, 20.0)
            .at_bus(1)
            .with_cost_curve(CostCurve::piecewise(20.0, 100.0, 400.0, 2400.0)),
    )
    .unwrap();
    sys.add_generator(
        Generator::new(2, "G2", GeneratorType::Thermal, 150.0, 30.0)
            .at_bus(2)
            .with_cost_curve(CostCurve::piecewise(30.0, 150.0, 900.0, 5400.0)),
    )
    .unwrap();
    sys.add_generator(
        Generator::new(3, "G3", GeneratorType::Thermal, 50.0, 10.0)
            .at_bus(3)
            .with_cost_curve(CostCurve::piecewise(50.0, 200.0, 2500.0, 12500.0)),
    )
    .unwrap();
    sys
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("test-data")
        .join("golden")
        .join("dispatch")
}

#[test]
fn unit_commitment_24hr_meets_load() {
    let raw = std::fs::read_to_string(golden_dir().join("unit-commitment-24hr.json"))
        .expect("read uc golden");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("parse uc golden");
    let loads: Vec<f64> = serde_json::from_value(v["load_profile_mw"].clone()).unwrap();
    let r = unit_commitment(&uc_system(), &loads);
    for (k, &load) in loads.iter().enumerate() {
        let total: f64 = r.outputs.iter().map(|o| o[k]).sum();
        assert!(
            total >= load - 1.0,
            "hour {k}: total={total} < load={load}"
        );
    }
}

#[derive(serde::Deserialize)]
struct EdGolden {
    test_loads_mw: Vec<LoadCase>,
}

#[derive(serde::Deserialize)]
struct LoadCase {
    load_mw: f64,
    expected_outputs_mw: Vec<f64>,
    expected_lambda: f64,
    tolerance_mw: f64,
}

#[test]
fn economic_dispatch_5gen_matches_golden() {
    let raw = std::fs::read_to_string(golden_dir().join("economic-dispatch-5gen.json"))
        .expect("read ed golden");
    let g: EdGolden = serde_json::from_str(&raw).expect("parse ed golden");
    for case in &g.test_loads_mw {
        let r = economic_dispatch(&ed_system(), case.load_mw).expect("dispatch");
        for (i, (&got, &want)) in r
            .generator_outputs_mw
            .iter()
            .zip(case.expected_outputs_mw.iter())
            .enumerate()
        {
            assert!(
                (got - want).abs() < case.tolerance_mw,
                "load {}: gen {}: got {}, want {}",
                case.load_mw, i, got, want
            );
        }
        assert!(
            (r.marginal_cost_dollar_per_mwh - case.expected_lambda).abs() < 1.0,
            "load {}: lambda = {}, want {}",
            case.load_mw,
            r.marginal_cost_dollar_per_mwh,
            case.expected_lambda
        );
    }
}
