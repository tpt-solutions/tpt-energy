//! Golden-value validation for battery SoC cycling.

use std::path::PathBuf;
use tpt_nrg_battery::BatteryStorage;

#[derive(serde::Deserialize)]
struct Golden {
    energy_capacity_mwh: f64,
    power_rating_mw: f64,
    round_trip_efficiency: f64,
    min_soc: f64,
    initial_soc: f64,
    steps: Vec<Step>,
}

#[derive(serde::Deserialize)]
struct Step {
    op: String,
    power_mw: f64,
    duration_h: f64,
    expected_out_mwh: Option<f64>,
    expected_stored_mwh: Option<f64>,
    expected_soc_after: f64,
    tolerance: f64,
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("test-data")
        .join("golden")
        .join("storage")
        .join("battery-soc-cycling.json")
}

#[test]
fn battery_soc_cycling_matches_golden() {
    let raw = std::fs::read_to_string(golden_path()).expect("read battery golden");
    let g: Golden = serde_json::from_str(&raw).expect("parse battery golden");
    let mut b = BatteryStorage::new(
        g.energy_capacity_mwh,
        g.power_rating_mw,
        g.round_trip_efficiency,
    )
    .with_soc(g.min_soc, g.initial_soc);

    for (i, step) in g.steps.iter().enumerate() {
        match step.op.as_str() {
            "discharge" => {
                let out = b.discharge(step.power_mw, step.duration_h).expect("discharge");
                if let Some(want) = step.expected_out_mwh {
                    assert!(
                        (out - want).abs() < step.tolerance,
                        "step {i}: out = {out}, want {want}"
                    );
                }
            }
            "charge" => {
                let stored = b.charge(step.power_mw, step.duration_h).expect("charge");
                if let Some(want) = step.expected_stored_mwh {
                    assert!(
                        (stored - want).abs() < step.tolerance,
                        "step {i}: stored = {stored}, want {want}"
                    );
                }
            }
            other => panic!("unknown op {other}"),
        }
        assert!(
            (b.soc - step.expected_soc_after).abs() < step.tolerance,
            "step {i}: SoC = {}, want {}",
            b.soc,
            step.expected_soc_after
        );
    }
}
