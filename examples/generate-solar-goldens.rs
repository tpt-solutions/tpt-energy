//! Generate solar-position golden values by running tpt-nrg-solar at
//! reference timestamps. Run with:
//!   cargo run --manifest-path examples/Cargo.toml --bin generate-solar-goldens

use chrono::{DateTime, Utc};
use tpt_nrg_solar::{SolarModel, SolarPosition};


/// Round to 9 decimal places so regenerated fixtures are byte-stable
/// across platforms (libm ulp differences must not fail the drift guard).
fn round9(x: f64) -> f64 {
    (x * 1.0e9).round() / 1.0e9
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_path = here
        .join("..")
        .join("test-data")
        .join("golden")
        .join("solar")
        .join("nrel-spa-zenith.json");
    std::fs::create_dir_all(out_path.parent().unwrap())?;

    let model = SolarModel::new(51.5, -0.45, 25.0, 0.0);
    let timestamps = [
        "2026-06-21T11:00:00Z",
        "2026-06-21T12:00:00Z",
        "2026-06-21T13:00:00Z",
        "2026-12-21T12:00:00Z",
        "2026-03-20T12:00:00Z",
    ];
    let entries: Vec<_> = timestamps
        .iter()
        .map(|s| {
            let dt: DateTime<Utc> = s.parse().unwrap();
            let pos: SolarPosition = model.solar_position(dt);
            serde_json::json!({
                "timestamp": s,
                "zenith_deg": round9(pos.zenith_deg),
                "azimuth_deg": round9(pos.azimuth_deg),
                "altitude_deg": round9(pos.altitude_deg),
                "declination_deg": round9(pos.declination_deg),
                "hour_angle_deg": round9(pos.hour_angle_deg),
                "air_mass": if pos.air_mass.is_finite() { round9(pos.air_mass) } else { f64::NAN },
            })
        })
        .collect();

    let json = serde_json::json!({
        "site": {
            "name": "London (Heathrow)",
            "latitude_deg": 51.5,
            "longitude_deg": -0.45,
            "altitude_m": 25.0,
            "timezone_offset_hours": 0.0
        },
        "tolerance": {
            "zenith_deg": 0.05,
            "azimuth_deg": 0.5,
            "altitude_deg": 0.05,
            "declination_deg": 0.05,
            "air_mass_abs": 0.01
        },
        "description": "Solar-position reference values for the tpt-nrg-solar low-precision algorithm (Meeus/NOAA-style). Stable to <0.01° within the algorithm's stated accuracy.",
        "samples": entries
    });
    std::fs::write(&out_path, serde_json::to_string_pretty(&json)?)?;
    println!("wrote {}", out_path.display());
    Ok(())
}
