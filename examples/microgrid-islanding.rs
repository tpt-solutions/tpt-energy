//! Microgrid islanding event: simulate a grid outage and the battery
//! dispatch response.
//!
//! Run with: `cargo run --example microgrid-islanding`

use tpt_nrg_battery::BatteryStorage;
use tpt_nrg_islanding::{transition_to_island, IslandingDetector};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let detector = IslandingDetector::new();
    println!("Detecting islanding at 0.85 pu, 59.4 Hz, RoCoF 0.2 Hz/s:");
    let is_island = detector.detect_islanding(0.85, 59.4, 0.2);
    println!("  islanding detected: {is_island}");

    println!("\nTransitioning to island:");
    let result = transition_to_island(
        /* total_load_mw */ 10.0,
        /* available_generation_mw */ 4.0, // solar+wind during outage
        /* storage_available_mw */ 5.0,
        /* nominal_voltage_pu */ 1.0,
        /* nominal_frequency_hz */ 60.0,
    );
    println!("  success={}, load_shed={:.2} MW, storage_dispatch={:.2} MW",
        result.success, result.load_shed_mw, result.storage_dispatch_mw);

    println!("\nSimulating 1 hour of islanded operation:");
    let mut battery = BatteryStorage::new(20.0, 5.0, 0.92).with_soc(0.1, 0.8);
    println!("  Initial SoC: {:.1}%", battery.soc * 100.0);
    for t in 0..12 {
        let out = battery.discharge(5.0, 5.0 / 60.0)?;
        println!("  T+{:>2} min: dispatched {:.3} MWh, SoC = {:.1}%",
            t * 5, out, battery.soc * 100.0);
    }
    Ok(())
}
