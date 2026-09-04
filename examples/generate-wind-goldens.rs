//! Generate wind-power golden values (Weibull PDF, Jensen wake deficit) by
//! running tpt-nrg-wind. Run with:
//!   cargo run --manifest-path examples/Cargo.toml --bin generate-wind-goldens

use tpt_nrg_wind::{WakeModel, WindFarm, WindModel, WindTurbine};

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
        .map(|&v| (v, wm.weibull_probability(v, k, c)))
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

    // ---- Jensen single-wake deficit ----
    let d: f64 = 80.0;
    let ct: f64 = 0.8;
    let k_wake: f64 = 0.075;
    let v0: f64 = 10.0;
    let wake_samples: Vec<_> = [80.0_f64, 160.0, 320.0, 640.0, 1280.0]
        .iter()
        .map(|&dist| {
            let dw = d + 2.0 * k_wake * dist;
            let factor = 1.0 - (1.0 - ct * (d / dw).powi(2)).max(0.0_f64).sqrt();
            (dist, v0 * (1.0 - factor))
        })
        .collect();

    // ---- Farm-level: 2-turbine line, 5D spacing, Jensen ----
    let turbine = WindTurbine::new("Generic 2MW", d, 2.0, 3.0, 12.0, 25.0);
    let mut farm = WindFarm::new(WakeModel::JensenPark, 90.0).with_wake_decay(0.075);
    farm.push(0.0, 0.0, turbine.clone());
    farm.push(5.0 * d, 0.0, turbine.clone()); // 5D downstream
    let eff_2t = farm.effective_wind_speeds(v0);

    let json = serde_json::json!({
        "weibull_probability": {
            "shape_k": k,
            "scale_c_mps": c,
            "samples_mps": weibull_samples.iter().map(|(v,p)| serde_json::json!({
                "v_mps": v, "pdf": p
            })).collect::<Vec<_>>(),
            "integral_0_to_25": weibull_integral,
            "tolerance_abs": 1e-12
        },
        "jensen_wake_deficit": {
            "rotor_diameter_m": d,
            "thrust_coefficient": ct,
            "wake_decay_k": k_wake,
            "upstream_wind_speed_mps": v0,
            "downstream_samples_mps": wake_samples.iter().map(|(dist,v)| serde_json::json!({
                "distance_d_m": dist, "v_downstream_mps": v, "deficit_factor": 1.0 - v / v0
            })).collect::<Vec<_>>(),
            "tolerance_abs": 1e-6
        },
        "farm_two_turbines_5d_spacing": {
            "upstream_wind_speed_mps": v0,
            "effective_wind_speeds_mps": eff_2t,
            "tolerance_abs": 1e-6
        }
    });
    let out = out_dir.join("weibull-probability.json");
    std::fs::write(&out, serde_json::to_string_pretty(&json)?)?;
    println!("wrote {}", out.display());

    // Also write the jensen-wake-deficit.json alias for the todo requirement
    // of having a separate file by that name.
    let wake_only = serde_json::json!({
        "rotor_diameter_m": d,
        "thrust_coefficient": ct,
        "wake_decay_k": k_wake,
        "upstream_wind_speed_mps": v0,
        "downstream_samples_mps": wake_samples.iter().map(|(dist,v)| serde_json::json!({
            "distance_d_m": dist, "v_downstream_mps": v, "deficit_factor": 1.0 - v / v0
        })).collect::<Vec<_>>(),
        "tolerance_abs": 1e-6
    });
    let out2 = out_dir.join("jensen-wake-deficit.json");
    std::fs::write(&out2, serde_json::to_string_pretty(&wake_only)?)?;
    println!("wrote {}", out2.display());
    Ok(())
}
