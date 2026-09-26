//! Golden-value validation for tpt-nrg-wind: Weibull PDF integral, Jensen
//! wake deficit, Frandsen wake deficit, and eddy-viscosity wake deficit.

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

#[test]
fn jensen_wake_deficit_matches_golden() {
    let raw =
        std::fs::read_to_string(golden_path("jensen-wake-deficit.json")).expect("read wake golden");
    let g: WakeGolden = serde_json::from_str(&raw).expect("parse wake golden");
    validate_wake_golden(WakeModel::JensenPark, &g);
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

fn validate_wake_golden(model: WakeModel, g: &WakeGolden) {
    // Exercise the crate itself: a single upstream turbine wakes one
    // directly-aligned downstream turbine at each sample distance.
    for s in &g.downstream_samples_mps {
        let turbine = WindTurbine::new("T", g.rotor_diameter_m, 2.0, 3.0, 12.0, 25.0)
            .with_thrust_coefficient(g.thrust_coefficient);
        let mut farm = WindFarm::new(model, 90.0).with_wake_decay(g.wake_decay_k);
        farm.push(0.0, 0.0, turbine.clone());
        farm.push(s.distance_d_m, 0.0, turbine);
        let eff = farm.effective_wind_speeds(g.upstream_wind_speed_mps);
        assert!(
            (eff[1] - s.v_downstream_mps).abs() < g.tolerance_abs,
            "{model:?} d = {}: v = {}, want {}",
            s.distance_d_m,
            eff[1],
            s.v_downstream_mps
        );
        let deficit = 1.0 - eff[1] / g.upstream_wind_speed_mps;
        assert!(
            (deficit - s.deficit_factor).abs() < g.tolerance_abs,
            "{model:?} d = {}: deficit = {}, want {}",
            s.distance_d_m,
            deficit,
            s.deficit_factor
        );
    }
}

#[test]
fn frandsen_wake_deficit_matches_golden() {
    let raw = std::fs::read_to_string(golden_path("frandsen-wake-deficit.json"))
        .expect("read frandsen golden");
    let g: WakeGolden = serde_json::from_str(&raw).expect("parse frandsen golden");
    validate_wake_golden(WakeModel::Frandsen, &g);
}

#[test]
fn eddy_viscosity_wake_deficit_matches_golden() {
    let raw = std::fs::read_to_string(golden_path("eddy-viscosity-wake-deficit.json"))
        .expect("read eddy-viscosity golden");
    let g: WakeGolden = serde_json::from_str(&raw).expect("parse eddy-viscosity golden");
    validate_wake_golden(WakeModel::EddyViscosity, &g);
}

#[test]
fn wake_models_agree_on_qualitative_behaviour() {
    // All three models must produce a wake: the downstream turbine is
    // slower than the free stream, and the deficit shrinks with distance.
    for model in [
        WakeModel::JensenPark,
        WakeModel::Frandsen,
        WakeModel::EddyViscosity,
    ] {
        let turbine = WindTurbine::new("T", 80.0, 2.0, 3.0, 12.0, 25.0);
        let mut close = WindFarm::new(model, 90.0).with_wake_decay(0.05);
        close.push(0.0, 0.0, turbine.clone());
        close.push(240.0, 0.0, turbine.clone()); // 3D
        let mut far = WindFarm::new(model, 90.0).with_wake_decay(0.05);
        far.push(0.0, 0.0, turbine.clone());
        far.push(1280.0, 0.0, turbine); // 16D
        let v_close = close.effective_wind_speeds(10.0);
        let v_far = far.effective_wind_speeds(10.0);
        assert!(
            v_close[1] < 10.0,
            "{model:?}: near turbine must be waked, got {}",
            v_close[1]
        );
        assert!(
            v_far[1] > v_close[1],
            "{model:?}: deficit must shrink downstream, near {} far {}",
            v_close[1],
            v_far[1]
        );
        assert!(v_far[1] < 10.0, "{model:?}: far wake must persist");
    }
}
