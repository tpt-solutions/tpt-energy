# `tpt-nrg-lcoe` · `tpt-nrg-market` · `tpt-nrg-carbon`

Economics crates.

## `tpt-nrg-lcoe`

Levelized cost of energy, NPV, and IRR.

```rust,no_run
use tpt_nrg_lcoe::{LcoeInputs, levelized_cost_of_energy, net_present_value, internal_rate_of_return};

let i = LcoeInputs::new(
    1.0e9,    // CAPEX
    1.0e7,    // annual fixed O&M
    0.0,      // variable O&M $/MWh
    0.0,      // fuel $/MWh
    100.0 * 0.5 * 8760.0, // annual energy (100 MW @ 50% CF)
    30,       // lifetime
    0.07,     // discount rate
    0.5,      // capacity factor
);

let lcoe = levelized_cost_of_energy(&i);                  // ~$200/MWh
let npv  = net_present_value(&[-1000.0, 1100.0], 0.10);   // ~0
let irr  = internal_rate_of_return(&[-1000.0, 1100.0]);    // Some(0.10)
```

## `tpt-nrg-market`

Market-signal modelling on top of `tpt-nrg-timeseries`.

```rust,no_run
use tpt_nrg_market::{MarketSignal, MarketType};
use tpt_nrg_timeseries::UniformTimeSeries;
use chrono::TimeZone;

let ts = UniformTimeSeries::new("p", "$/MWh",
    chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
    3600,
    vec![20.0, 18.0, 15.0, /* ... */]);
let sig = MarketSignal::new(MarketType::DayAhead, ts);

println!("mean = {:.2}", sig.mean_price());
println!("peak = {:.2}", sig.peak_price());
println!("spread = {:.2}", sig.price_spread());
```

## `tpt-nrg-carbon`

Dispatch-weighted carbon intensity.

```rust,no_run
use tpt_nrg_core::EnergySystem;
use tpt_nrg_carbon::{carbon_intensity, total_emissions_tonnes};

let system = EnergySystem::from_json(/* ... */)?;
let ci = carbon_intensity(&system);     // kg CO2 / MWh (weighted by output)
let e  = total_emissions_tonnes(&system, 1.0); // t CO2 over 1 hour
```

Emission factors (kg CO₂ / MWh, lifecycle midpoints):

| Fuel | Factor |
|------|--------|
| Thermal (gas/coal avg) | 900 |
| Hydro (reservoir, lifecycle) | 4 |
| Wind | 11 |
| Solar | 45 |
| Nuclear | 12 |
| Geothermal | 38 |
