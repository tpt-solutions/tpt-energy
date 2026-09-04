# `tpt-nrg-der` · `tpt-nrg-islanding` · `tpt-nrg-vpp`

Microgrid control crates.

## `tpt-nrg-der`

DER asset aggregation and controller state.

```rust,no_run
use tpt_nrg_der::{DerAsset, MicrogridController, ControlStrategy};

let mut c = MicrogridController::new(ControlStrategy::GridFollowing, 5.0);
c.add_asset(DerAsset::Solar { capacity_mw: 10.0, output_fraction: 0.6 });
c.add_asset(DerAsset::Battery { power_rating_mw: 5.0, energy_capacity_mwh: 10.0, state_of_charge: 0.5 });
c.add_asset(DerAsset::Load    { rated_mw: 1.0, demand_fraction: 1.0 });

let net = c.net_balance_mw(); // generation - demand
```

## `tpt-nrg-islanding`

Loss-of-mains detection (IEEE 1547 thresholds), controlled transition to islanded operation, and resynchronization with the main grid.

```rust,no_run
use tpt_nrg_islanding::{IslandingDetector, transition_to_island, resynchronize};

let det = IslandingDetector::default();
let islanded = det.detect_islanding(0.85, 60.0, 0.1); // voltage sag

let r = transition_to_island(10.0, 4.0, 3.0, 1.0, 60.0);
// r.success, r.load_shed_mw, r.storage_dispatch_mw

let sync = resynchronize(1.0, 1.01, 0.0, 0.05, 60.0, 60.05, 0.05, 0.05, 0.1);
// sync.success, sync.voltage_error_pu, sync.frequency_error_hz
```

## `tpt-nrg-vpp`

Virtual power plant aggregation with diversity adjustment and pro-rata dispatch.

```rust,no_run
use tpt_nrg_vpp::{VirtualPowerPlant, AggregationModel};

let vpp = VirtualPowerPlant::new(
    vec![(10.0, 5.0, 2.0, 2.0), (20.0, 10.0, 5.0, 3.0)],
    AggregationModel::DiversityAdjusted { factor: 0.9 },
);

let plan = vpp.dispatch_assets(3.0);
// plan.per_asset_delta_mw, plan.delivered_mw, plan.unfulfilled_mw
```
