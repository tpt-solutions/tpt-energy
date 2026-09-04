# `tpt-nrg-battery` · `tpt-nrg-hydrogen`

Energy-storage crates.

## `tpt-nrg-battery`

State-of-charge tracking with round-trip efficiency and capacity-fade degradation.

```rust,no_run
use tpt_nrg_battery::{BatteryStorage, DegradationModel};

let mut b = BatteryStorage::new(100.0, 50.0, 0.9) // 100 MWh, 50 MW, 90% RT
    .with_soc(0.05, 0.50)                         // 5% min, start 50%
    .with_degradation(DegradationModel::li_ion());

let stored = b.charge(50.0, 1.0)?;    // 50 MW for 1 h
let out    = b.discharge(50.0, 1.0)?; // 50 MW for 1 h
let cf     = b.capacity_factor();     // 0.5..=1.0, factoring degradation
```

The crate integrates with `tpt-eng-materials` via the `substrate` feature for NMC-811 specific-capacity lookups (used when sizing new cells).

## `tpt-nrg-hydrogen`

PEM electrolyzer + PEM fuel cell + tank.

```rust,no_run
use tpt_nrg_hydrogen::{Electrolyzer, FuelCell, HydrogenSystem};

let mut sys = HydrogenSystem::new(
    Electrolyzer::new(50.0, 10.0),   // 50 kWh/kg H2, 10 MW
    FuelCell::new(17.0, 5.0),         // 17 kWh/kg H2, 5 MW
    100.0,                            // 100 kg tank
);

let h2_produced = sys.charge(1.0);          // 1 MWh in -> 20 kg H2
let mwh_out     = sys.discharge(5.0, 0.1);  // 5 MW for 0.1 h -> 0.34 MWh (capped by H2)
let rt_eff      = sys.round_trip_efficiency(); // ~34% ideal
```

The tank capacity clamps production; the fuel cell power rating clamps discharge.
