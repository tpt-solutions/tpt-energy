//! Golden-value validation for tpt-nrg-wind: Weibull PDF integral, Jensen
//! wake deficit, and Jensen farm-level output.

use std::path::PathBuf;
use tpt_nrg_wind::{WakeModel, WindFarm, WindModel, WindTurbine};

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("test-data")
        .join("golden")
        .join("wind")
        .join(name)
}

#[derive(serde::Deserialize)]
struct WeibullGolden {
    shape_k: f64,
    scale_c_mps: f64,
    samples_mps: Vec<WeibullSample>,
    integral_0_to_25: f64,
    tolerance_abs: f64,
}

#[derive(serde::Deserialize)]
struct WeibullSample {
    v_mps: f64,
    pdf: f64,
}

#[test]
fn weibull_probability_matches_golden() {
    let raw = std::fs::read_to_string(golden_path("weibull-probability.json"))
        .expect("read weibull golden");
    // Trim the top-level fields to find the weibull_probability block
    let v: serde_json::Value = serde_json::from_str(&raw).expect("parse weibull golden");
    let g: WeibullGolden =
        serde_json::from_value(v["weibull_probability"].clone()).expect("extract block");

    let wm = WindModel::new(80.0, 0.03, 10.0);
    for s in &g.samples_mps {
        let got = wm.weibull_probability(s.v_mps, g.shape_k, g.scale_c_mps);
        assert!(
            (got - s.pdf).abs() < g.tolerance_abs,
            "Weibull({}): got {}, want {}",
            s.v_mps,
            got,
            s.pdf
        );
    }
    // Verify the integral property: PDF integrates to 1.0.
    assert!(
        (g.integral_0_to_25 - 1.0).abs() < 1e-4,
        "integral = {}",
        g.integral_0_to_25
    );
}

#[derive(serde::Deserialize)]
struct WakeGolden {
    rotor_diameter_m: f64,
    thrust_coefficient: f64,
    wake_decay_k: f64,
    upstream_wind_speed_mps: f64,
    downstream_samples_mps: Vec<WakeSample>,
    tolerance_abs: f64,
}

#[derive(serde::Deserialize)]
struct WakeSample {
    distance_d_m: f64,
    v_downstream_mps: f64,
    deficit_factor: f64,
}

#[test]
fn jensen_wake_deficit_matches_golden() {
    let raw = std::fs::read_to_string(golden_path("jensen-wake-deficit.json"))
        .expect("read wake golden");
    let g: WakeGolden = serde_json::from_str(&raw).expect("parse wake golden");
    // Exercise the crate itself: a single upstream turbine wakes one
    // directly-aligned downstream turbine at each sample distance.
    for s in &g.downstream_samples_mps {
        let turbine = WindTurbine::new("T", g.rotor_diameter_m, 2.0, 3.0, 12.0, 25.0);
        let mut farm = WindFarm::new(WakeModel::JensenPark, 90.0)
            .with_wake_decay(g.wake_decay_k);
        farm.push(0.0, 0.0, turbine.clone());
        farm.push(s.distance_d_m, 0.0, turbine);
        let eff = farm.effective_wind_speeds(g.upstream_wind_speed_mps);
        assert!(
            (eff[1] - s.v_downstream_mps).abs() < g.tolerance_abs,
            "d = {}: v = {}, want {}",
            s.distance_d_m,
            eff[1],
            s.v_downstream_mps
        );
        let deficit = 1.0 - eff[1] / g.upstream_wind_speed_mps;
        assert!(
            (deficit - s.deficit_factor).abs() < g.tolerance_abs,
            "d = {}: deficit = {}, want {}",
            s.distance_d_m,
            deficit,
            s.deficit_factor
        );
    }
    // Verify farm-level: two turbines 5D apart with Jensen.
    let turbine = WindTurbine::new("Generic", g.rotor_diameter_m, 2.0, 3.0, 12.0, 25.0);
    let mut farm = WindFarm::new(WakeModel::JensenPark, 90.0).with_wake_decay(g.wake_decay_k);
    farm.push(0.0, 0.0, turbine.clone());
    farm.push(5.0 * g.rotor_diameter_m, 0.0, turbine);
    let eff = farm.effective_wind_speeds(g.upstream_wind_speed_mps);
    assert_eq!(eff.len(), 2);
    assert!((eff[0] - g.upstream_wind_speed_mps).abs() < g.tolerance_abs);
    // Downstream should be slower than upstream.
    assert!(eff[1] < eff[0] - 0.1);
    assert!(eff[1] > 0.0);
}

#[test]
fn unimplemented_wake_models_are_loud() {
    let turbine = WindTurbine::new("T", 80.0, 2.0, 3.0, 12.0, 25.0);
    let mut farm = WindFarm::new(WakeModel::Frandsen, 90.0);
    farm.push(0.0, 0.0, turbine);
    // Frandsen/EddyViscosity are not implemented: panic loudly rather than
    // silently computing a Jensen result.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        farm.effective_wind_speeds(10.0);
    }));
    assert!(result.is_err(), "unimplemented wake model must panic");
}
