//! Golden-value validation for battery SoC cycling.
//!
//! Reads `test-data/golden/storage/battery-soc-cycling.json` and replays the
//! charge/discharge schedule against [`BatteryStorage`], checking the SoC
//! after every step and the cumulative-throughput delta.

use std::path::PathBuf;
use tpt_nrg_battery::BatteryStorage;

/// Full golden fixture layout.
#[derive(serde::Deserialize)]
struct Golden {
    config: Config,
    cycles: Vec<Cycle>,
    tolerance: Tolerance,
}

#[derive(serde::Deserialize)]
struct Config {
    energy_capacity_mwh: f64,
    power_rating_mw: f64,
    round_trip_efficiency: f64,
    min_soc: f64,
    initial_soc: f64,
}

#[derive(serde::Deserialize)]
struct Cycle {
    action: String,
    duration_h: f64,
    power_mw: f64,
    expected_soc_after: f64,
    expected_throughput_mwh_delta: f64,
}

#[derive(serde::Deserialize)]
struct Tolerance {
    soc_abs: f64,
    throughput_abs: f64,
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
        g.config.energy_capacity_mwh,
        g.config.power_rating_mw,
        g.config.round_trip_efficiency,
    )
    .with_soc(g.config.min_soc, g.config.initial_soc);

    for (i, cyc) in g.cycles.iter().enumerate() {
        let throughput_before = b.cumulative_throughput_mwh;
        match cyc.action.as_str() {
            "charge" => {
                b.charge(cyc.power_mw, cyc.duration_h).expect("charge");
            }
            "discharge" => {
                b.discharge(cyc.power_mw, cyc.duration_h)
                    .expect("discharge");
            }
            other => panic!("step {i}: unknown action {other}"),
        }
        assert!(
            (b.soc - cyc.expected_soc_after).abs() < g.tolerance.soc_abs,
            "step {i}: SoC = {:.5}, want {:.5}",
            b.soc,
            cyc.expected_soc_after
        );
        let delta = b.cumulative_throughput_mwh - throughput_before;
        assert!(
            (delta - cyc.expected_throughput_mwh_delta).abs() < g.tolerance.throughput_abs,
            "step {i}: throughput delta = {delta:.3}, want {:.3}",
            cyc.expected_throughput_mwh_delta
        );
    }
}
