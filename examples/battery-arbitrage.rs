//! Battery arbitrage from a 24-hour price forecast.
//!
//! Run with: `cargo run --example battery-arbitrage`

use tpt_nrg_battery::BatteryStorage;
use tpt_nrg_economic_dispatch::storage_arbitrage;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 24-hour price profile ($/MWh)
    let prices: Vec<f64> = vec![
        25.0, 22.0, 20.0, 18.0, 18.0, 20.0,  // night
        28.0, 40.0, 55.0, 60.0, 58.0, 55.0,  // morning ramp
        50.0, 48.0, 45.0, 50.0, 65.0, 90.0,  // afternoon peak
        110.0, 95.0, 70.0, 50.0, 35.0, 28.0,  // evening
    ];
    let plan = storage_arbitrage(&prices, 1.0, 100.0, 50.0, 0.9);
    println!("Battery arbitrage plan:");
    println!("hour, price, charge_mw, discharge_mw, soc_mwh, net_revenue");
    let mut cum = 0.0;
    for (i, _) in prices.iter().enumerate() {
        let revenue_change = plan.discharge_mw[i] * prices[i]
            - plan.charge_mw[i] * prices[i];
        cum += revenue_change;
        println!("{:>2}, {:.2}, {:.2}, {:.2}, {:.2}, {:.2}",
            i, prices[i], plan.charge_mw[i], plan.discharge_mw[i],
            plan.state_of_charge[i + 1], cum);
    }
    println!("\nNet revenue over 24h: ${:.2}", plan.net_revenue_dollar);

    let _ = BatteryStorage::new(100.0, 50.0, 0.9);
    Ok(())
}
