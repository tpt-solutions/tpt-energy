# `tpt-nrg-hydro` · `tpt-nrg-load` · `tpt-nrg-thermal-storage`

Three small crates grouped here for brevity.

## `tpt-nrg-hydro`

Run-of-river and reservoir hydro power from head × flow × efficiency.

```rust,no_run
use tpt_nrg_hydro::HydroPlant;

let plant = HydroPlant::new(50.0, 0.90, 100.0); // 50 m head, 90% eff, 100 m³/s
let mw = plant.power_output_mw(80.0);          // ~35.3 MW
```

The crate integrates with `GeneratorType::Hydro` in `tpt-nrg-core`.

## `tpt-nrg-load`

Load forecasting and demand response.

```rust,no_run
use tpt_nrg_load::LoadModel;

let m = LoadModel::new(100.0)                   // 100 MW base
    .with_temperature_sensitivity(2.0, 20.0)    // +2 MW / °C from 20°C
    .with_price_elasticity(-0.5);               // -0.5 MW per $/MWh

let forecast = m.forecast_load(0, 25.0, 50.0);  // hour-of-week, temp, price
let dr       = m.demand_response(100.0, 50.0); // reduction at $100 vs $50 ref
```

Hour-of-week is `0..168` (Mon=0..23, Sun=168..167). A weekly shape can be attached with `with_weekly_shape`.

## `tpt-nrg-thermal-storage`

Thermal energy storage (molten salt, chilled water, PCM, etc.).

```rust,no_run
use tpt_nrg_thermal_storage::ThermalStorage;

let mut t = ThermalStorage::new(100.0, 50.0, 0.80) // 100 MWh_th, 50 MW_th, 80% RT
    .with_soc_bounds(0.05, 0.20);

let stored = t.charge(40.0, 1.0);     // 40 MW for 1 h
let out    = t.discharge(50.0, 1.0);  // 50 MW for 1 h (clamped by SoC)
```

The state update applies `sqrt(η)` to charge and `1/sqrt(η)` to discharge so the round-trip efficiency is exactly `η`.
