//! End-to-end Energy Cycle simulation: material degradation -> device
//! physics -> grid dispatch.
//!
//! This example walks through the full loop:
//!
//! 1. **Resource**: a 50 MWp PV plant (clear-sky model) and a 50 MW
//!    wind farm (Jensen wake model) generate power over a 24-hour
//!    horizon.
//! 2. **Storage**: a 100 MWh Li-ion battery charges from PV excess and
//!    discharges into the evening peak. The battery tracks degradation
//!    from cycles and calendar age.
//! 3. **Grid**: an IEEE 14-bus system absorbs the storage dispatch
//!    alongside a fixed load profile; Newton-Raphson power flow gives
//!    per-bus voltages and losses.
//! 4. **Dispatch**: a 5-generator economic dispatch covers the
//!    remaining load at minimum cost, with the battery bidding in.
//! 5. **Economics**: the per-MWh LCOE of the new PV+battery asset is
//!    compared against the marginal cost from the dispatch.
//!
//! Run with: `cargo run --example energy-cycle`.

use std::error::Error;
use chrono::{Duration, TimeZone, Utc};

use tpt_nrg_solar::{PvPlant, PvPlantConfig, SolarModel};
use tpt_nrg_wind::{WakeModel, WindFarm, WindModel, WindTurbine};
use tpt_nrg_battery::{BatteryStorage, DegradationModel};
use tpt_nrg_core::EnergySystem;
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};
use tpt_nrg_economic_dispatch::economic_dispatch;
use tpt_nrg_lcoe::{LcoeInputs, levelized_cost_of_energy};
use tpt_nrg_carbon::carbon_intensity;

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== TPT Energy: end-to-end Energy Cycle ===\n");

    // ----------------------------------------------------------------
    // 1. Resource: solar + wind profiles
    // ----------------------------------------------------------------
    let site = SolarModel::new(40.0, -105.0, 1600.0, -7.0);
    let pv = PvPlant::new(site, PvPlantConfig::new(50.0, 30.0, 180.0, 25.0));

    let turbine = WindTurbine::new("Generic 2MW", 80.0, 2.0, 3.0, 12.0, 25.0);
    let mut wind_farm = WindFarm::new(WakeModel::JensenPark, 90.0).with_wake_decay(0.05);
    for i in 0..5 {
        wind_farm.push(i as f64 * 400.0, 0.0, turbine.clone());
    }
    let wind_speed = WindModel::new(80.0, 0.03, 10.0);

    let start = Utc.with_ymd_and_hms(2026, 6, 21, 0, 0, 0).unwrap();
    let mut solar_mw = Vec::new();
    let mut wind_mw = Vec::new();
    for h in 0..24 {
        let t = start + Duration::hours(h);
        solar_mw.push(pv.output_at(t).ac_power_mw);
        wind_mw.push(wind_farm.total_power_output(wind_speed.wind_speed_at_height(8.0)));
    }

    println!("Step 1: Resource (24-hour clear-sky + constant-wind profile)");
    let solar_total: f64 = solar_mw.iter().sum();
    let wind_total: f64 = wind_mw.iter().sum();
    println!("  solar: {:.1} MWh, peak {:.2} MW", solar_total, solar_mw.iter().cloned().fold(0.0_f64, f64::max));
    println!("  wind : {:.1} MWh, constant {:.2} MW", wind_total, wind_mw[12]);

    // ----------------------------------------------------------------
    // 2. Storage: battery cycles from PV excess
    // ----------------------------------------------------------------
    let mut battery = BatteryStorage::new(100.0, 25.0, 0.90)
        .with_soc(0.05, 0.20)
        .with_degradation(DegradationModel::li_ion());
    let load_mw: Vec<f64> = vec![30.0; 24]; // local microgrid load

    let mut battery_dispatch = vec![0.0; 24];
    for h in 0..24 {
        let supply = solar_mw[h] + wind_mw[h];
        let net = supply - load_mw[h];
        if net > 0.0 {
            let _ = battery.charge(net.min(25.0), 1.0);
        } else if net < 0.0 {
            let discharged = battery.discharge((-net).min(25.0), 1.0).unwrap_or(0.0);
            battery_dispatch[h] = discharged;
        }
    }

    println!("\nStep 2: Battery dispatch");
    println!("  final SoC: {:.1}%", battery.soc * 100.0);
    println!(
        "  cumulative throughput: {:.1} MWh",
        battery.cumulative_throughput_mwh
    );
    println!(
        "  capacity factor (degradation-adjusted): {:.3}",
        battery.capacity_factor()
    );

    // ----------------------------------------------------------------
    // 3. Grid: IEEE 14-bus power flow with the storage as a generator
    // ----------------------------------------------------------------
    let json_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test-data/ieee/ieee14.json");
    let mut system = EnergySystem::from_json_file(&json_path)?;

    // Inject the PV+battery as a "negative load" at bus 14 (load bus)
    // for the evening peak hour.
    let battery_evening_mw: f64 = battery_dispatch[18..21].iter().sum();
    if let Some(bus) = system.buses.iter_mut().find(|b| b.id == 14) {
        bus.load_mw = (bus.load_mw - battery_evening_mw).max(0.0);
    }

    let solver = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson);
    let pf = solver.solve(&system)?;
    println!("\nStep 3: Grid (Newton-Raphson, IEEE 14-bus)");
    println!("  converged: {} ({} iterations)", pf.converged, pf.iterations);
    println!("  total losses: {:.2} MW", pf.total_losses_mw);

    // ----------------------------------------------------------------
    // 4. Dispatch: 5-generator economic dispatch
    // ----------------------------------------------------------------
    let dispatch = economic_dispatch(&system, 250.0)?;
    println!("\nStep 4: Economic dispatch (250 MW load)");
    println!(
        "  total cost: ${:.0}/h",
        dispatch.total_cost_dollar_per_h
    );
    println!(
        "  marginal cost (lambda): ${:.2}/MWh",
        dispatch.marginal_cost_dollar_per_mwh
    );

    // ----------------------------------------------------------------
    // 5. Economics: LCOE of the PV+battery asset vs marginal price
    // ----------------------------------------------------------------
    let annual_energy = (solar_total + wind_total) * 365.0; // assume similar profile year-round
    let lcoe_inputs = LcoeInputs::new(
        75.0e6,         // CAPEX $75M (50 MWp PV + 100 MWh battery + BoP)
        1.5e6,          // annual fixed O&M $1.5M
        1.0,            // variable O&M $1/MWh
        0.0,            // no fuel
        annual_energy,
        25,             // 25-year lifetime
        0.07,           // 7% discount
        0.25,           // 25% capacity factor (mixed PV+wind)
    );
    let lcoe = levelized_cost_of_energy(&lcoe_inputs);
    println!("\nStep 5: Economics");
    println!("  PV+wind+battery LCOE: ${:.2}/MWh", lcoe);
    println!(
        "  dispatch lambda:      ${:.2}/MWh",
        dispatch.marginal_cost_dollar_per_mwh
    );
    println!(
        "  carbon intensity:     {:.1} kg CO2/MWh",
        carbon_intensity(&system)
    );

    println!("\n=== Energy Cycle complete ===");
    Ok(())
}
