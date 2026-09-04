//! Wind farm wake-loss analysis using the Jensen/PARK model.
//!
//! Run with: `cargo run --example wind-farm-wake`

use tpt_nrg_wind::{WakeModel, WindFarm, WindTurbine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 5 turbines, each rated 2 MW, in a single row 5D apart, wind from west
    let turbine = WindTurbine::new("Generic 2MW", 80.0, 2.0, 3.0, 12.0, 25.0);
    let mut farm = WindFarm::new(WakeModel::JensenPark, 90.0).with_wake_decay(0.05);
    for i in 0..5 {
        farm.push(i as f64 * 400.0, 0.0, turbine.clone());
    }

    for v_free in [6.0, 8.0, 10.0, 12.0, 14.0] {
        let v_eff = farm.effective_wind_speeds(v_free);
        let p_total = farm.total_power_output(v_free);
        let p_no_wake = turbine.power_at(v_free) * 5.0;
        println!("Free-stream {:.0} m/s: P_farm={:.2} MW, P_no_wake={:.2} MW, loss={:.1}%",
            v_free, p_total, p_no_wake, 100.0 * (1.0 - p_total / p_no_wake));
        for (i, v) in v_eff.iter().enumerate() {
            println!("  T{}: v_eff = {:.2} m/s, P = {:.2} MW",
                i + 1, v, turbine.power_at(*v));
        }
    }
    Ok(())
}
