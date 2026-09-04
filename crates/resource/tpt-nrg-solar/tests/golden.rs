//! Golden-value validation for solar position and PV output models.

use std::path::PathBuf;
use tpt_nrg_solar::{SolarModel, SolarPosition};

#[derive(serde::Deserialize)]
struct Golden {
    site: Site,
    samples: Vec<Sample>,
    tolerance: Tol,
}

#[derive(serde::Deserialize)]
struct Site {
    latitude_deg: f64,
    longitude_deg: f64,
    altitude_m: f64,
    #[allow(dead_code)]
    timezone_offset_hours: f64,
}

#[derive(serde::Deserialize)]
struct Tol {
    zenith_deg: f64,
    #[allow(dead_code)]
    azimuth_deg: f64,
    altitude_deg: f64,
    declination_deg: f64,
    #[allow(dead_code)]
    air_mass_abs: f64,
}

#[derive(serde::Deserialize)]
struct Sample {
    timestamp: String,
    zenith_deg: f64,
    #[allow(dead_code)]
    azimuth_deg: f64,
    altitude_deg: f64,
    declination_deg: f64,
    #[allow(dead_code)]
    air_mass: f64,
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("test-data")
        .join("golden")
        .join("solar")
        .join("nrel-spa-zenith.json")
}

fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

#[test]
fn solar_position_matches_golden() {
    let raw = std::fs::read_to_string(golden_path()).expect("read solar golden");
    let g: Golden = serde_json::from_str(&raw).expect("parse solar golden");
    let model = SolarModel::new(
        g.site.latitude_deg,
        g.site.longitude_deg,
        g.site.altitude_m,
        g.site.timezone_offset_hours,
    );
    for s in &g.samples {
        let dt: chrono::DateTime<chrono::Utc> = s.timestamp.parse().unwrap();
        let pos: SolarPosition = model.solar_position(dt);
        assert!(
            approx_eq(pos.zenith_deg, s.zenith_deg, g.tolerance.zenith_deg),
            "{} zenith: got {}, want {}",
            s.timestamp,
            pos.zenith_deg,
            s.zenith_deg
        );
        assert!(
            approx_eq(pos.altitude_deg, s.altitude_deg, g.tolerance.altitude_deg),
            "{} altitude: got {}, want {}",
            s.timestamp,
            pos.altitude_deg,
            s.altitude_deg
        );
        assert!(
            approx_eq(
                pos.declination_deg,
                s.declination_deg,
                g.tolerance.declination_deg
            ),
            "{} declination: got {}, want {}",
            s.timestamp,
            pos.declination_deg,
            s.declination_deg
        );
    }
}
