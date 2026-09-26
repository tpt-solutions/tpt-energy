//! Generate wind-power golden values (Weibull PDF, wake deficits for all
//! three models) by running tpt-nrg-wind. Run with:
//!   cargo run --manifest-path examples/Cargo.toml --bin generate-wind-goldens
//!
//! The wake fixtures are produced by calling the library itself, so the
//! fixtures cannot silently drift from the solvers.

use tpt_nrg_wind::{WakeModel, WindFarm, WindModel, WindTurbine};


/// Round to 9 decimal places so regenerated fixtures are byte-stable
/// across platforms (libm ulp differences must not fail the drift guard).
fn round9(x: f64) -> f64 {
    (x * 1.0e9).round() / 1.0e9
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = here.join("..").join("test-data").join("golden").join("wind");
    std::fs::create_dir_all(&out_dir)?;

    // ---- Weibull PDF ----
    let wm = WindModel::new(80.0, 0.03, 10.0);
    let k: f64 = 2.0;
    let c: f64 = 7.0;
    let weibull_samples: Vec<_> = [0.0_f64, 2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0]
        .iter()
        .map(|&v| (v, round9(wm.weibull_probability(v, k, c))))
        .collect();
    let weibull_integral: f64 = {
        // Trapezoidal integration on [0, 25] to verify the PDF integrates to 1.
        let n = 1000;
        let upper = 25.0;
        let dv = upper / n as f64;
        let mut total = 0.0;
        for i in 0..=n {
            let v = i as f64 * dv;
            let f = if i == 0 || i == n { 0.5 } else { 1.0 };
            total += f * wm.weibull_probability(v, k, c);
        }
        total * dv
    };

    let d: f64 = 80.0;
    let ct: f64 = 0.8;
    let k_wake: f64 = 0.075;
    let v0: f64 = 10.0;

    // ---- Single-wake deficits: one upstream turbine, one aligned
    // downstream turbine at each sample distance, per model. Values are
    // read back from the library's effective_wind_speeds. ----
    let distances = [80.0_f64, 160.0, 320.0, 640.0, 1280.0];
    let sample = |model: WakeModel| -> Vec<(f64, f64)> {
        distances
            .iter()
            .map(|&dist| {
                let turbine = WindTurbine::new("T", d, 2.0, 3.0, 12.0, 25.0)
                    .with_thrust_coefficient(ct);
                let mut farm = WindFarm::new(model, 90.0).with_wake_decay(k_wake);
                farm.push(0.0, 0.0, turbine.clone());
                farm.push(dist, 0.0, turbine);
                let eff = farm.effective_wind_speeds(v0);
                (dist, round9(eff[1]))
            })
            .collect()
    };
    let jensen_samples = sample(WakeModel::JensenPark);
    let frandsen_samples = sample(WakeModel::Frandsen);
    let eddy_samples = sample(WakeModel::EddyViscosity);

    let wake_block = |samples: &[(f64, f64)]| {
        serde_json::json!({
            "rotor_diameter_m": d,
            "thrust_coefficient": ct,
            "wake_decay_k": k_wake,
            "upstream_wind_speed_mps": v0,
            "downstream_samples_mps": samples.iter().map(|(dist, v)| serde_json::json!({
                "distance_d_m": dist, "v_downstream_mps": v, "deficit_factor": 1.0 - v / v0
            })).collect::<Vec<_>>(),
            "tolerance_abs": 1e-6
        })
    };

    // ---- Farm-level: 2-turbine line, 5D spacing, Jensen ----
    let turbine = WindTurbine::new("Generic 2MW", d, 2.0, 3.0, 12.0, 25.0);
    let mut farm = WindFarm::new(WakeModel::JensenPark, 90.0).with_wake_decay(k_wake);
    farm.push(0.0, 0.0, turbine.clone());
    farm.push(5.0 * d, 0.0, turbine); // 5D downstream
    let eff_2t: Vec<f64> = farm
        .effective_wind_speeds(v0)
        .iter()
        .map(|&v| round9(v))
        .collect();

    let json = serde_json::json!({
        "weibull_probability": {
            "shape_k": k,
            "scale_c_mps": c,
            "samples_mps": weibull_samples.iter().map(|(v,p)| serde_json::json!({
                "v_mps": v, "pdf": p
            })).collect::<Vec<_>>(),
            "integral_0_to_25": round9(weibull_integral),
            "tolerance_abs": 1e-6
        },
        "jensen_wake_deficit": wake_block(&jensen_samples),
        "frandsen_wake_deficit": wake_block(&frandsen_samples),
        "eddy_viscosity_wake_deficit": wake_block(&eddy_samples),
        "farm_two_turbines_5d_spacing": {
            "upstream_wind_speed_mps": v0,
            "effective_wind_speeds_mps": eff_2t,
            "tolerance_abs": 1e-6
        }
    });
    let out = out_dir.join("weibull-probability.json");
    std::fs::write(&out, serde_json::to_string_pretty(&json)?)?;
    println!("wrote {}", out.display());

    // Per-model fixture files expected by tests/golden.rs.
    for (name, samples) in [
        ("jensen-wake-deficit.json", &jensen_samples),
        ("frandsen-wake-deficit.json", &frandsen_samples),
        ("eddy-viscosity-wake-deficit.json", &eddy_samples),
    ] {
        let path = out_dir.join(name);
        std::fs::write(&path, serde_json::to_string_pretty(&wake_block(samples))?)?;
        println!("wrote {}", path.display());
    }
    Ok(())
}
