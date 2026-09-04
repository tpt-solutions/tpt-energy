//! Compute a 24-hour solar PV generation profile.
//!
//! Run with: `cargo run --example solar-farm-layout`

use chrono::{Duration, TimeZone, Utc};
use tpt_nrg_solar::{PvPlant, PvPlantConfig, SolarModel};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Site: Boulder, CO (40.0°N, -105.3°W, 1655 m, UTC-7)
    let site = SolarModel::new(40.0, -105.3, 1655.0, -7.0);
    // Plant: 100 MWp DC, 35° tilt, south-facing, 25°C ambient
    let cfg = PvPlantConfig::new(100.0, 35.0, 180.0, 25.0);
    let plant = PvPlant::new(site, cfg);

    println!("24-hour solar PV profile (Boulder, CO, summer solstice)");
    println!("hour, ac_mw, dc_mw, cell_temp_c, poa_w_per_m2");
    for hour in 0..24 {
        let t = Utc.with_ymd_and_hms(2026, 6, 21, hour, 0, 0).unwrap()
            + Duration::hours(12); // noon-ish at -105 longitude
        let out = plant.output_at(t);
        println!("{:02}, {:.3}, {:.3}, {:.2}, {:.1}",
            hour, out.ac_power_mw, out.dc_power_mw,
            out.cell_temperature_c, out.poa_w_per_m2);
    }
    Ok(())
}
