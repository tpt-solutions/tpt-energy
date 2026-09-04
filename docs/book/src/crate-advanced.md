# `tpt-nrg-state-estimation` · `tpt-nrg-protection` · `tpt-nrg-wasm`

Advanced grid features and the WASM binding crate.

## `tpt-nrg-state-estimation`

DC-approximation weighted-least-squares state estimator.

```rust,no_run
use tpt_nrg_core::EnergySystem;
use tpt_nrg_state_estimation::{run_dc_state_estimation, Measurement};

let system = EnergySystem::from_json(/* ... */)?;
let m = vec![Measurement::PowerInjection { bus: 2, value_mw: 50.0, variance: 1.0 }];
let r = run_dc_state_estimation(&system, &m);
// r.voltage_angles_rad, r.voltage_magnitudes_pu (1.0 pu in DC)
// r.residuals, r.objective
```

A full non-linear WLS estimator with bad-data detection is planned for a later phase (see Phase 6 note in `todo.md`).

## `tpt-nrg-protection`

IEC 60255-151 inverse-time overcurrent relays with coordination checks.

```rust,no_run
use tpt_nrg_protection::{Relay, RelayCurve, RelayEnd, check_coordination};

let primary = Relay {
    id: 1, branch_id: 1, end: RelayEnd::From,
    pickup_pu: 1.0, time_multiplier: 0.1, curve: RelayCurve::StandardInverse,
    time_delay_s: 0.0,
};
let backup = Relay {
    id: 2, branch_id: 1, end: RelayEnd::To,
    pickup_pu: 1.0, time_multiplier: 0.5, curve: RelayCurve::StandardInverse,
    time_delay_s: 0.3,
};
let ok = check_coordination(&primary, &backup, &[2.0, 5.0, 10.0, 20.0]);
// ok == true: backup trips at least 0.2 s after primary at every fault level
```

## `tpt-nrg-wasm`

JSON-driven power flow for in-browser use.

```rust,no_run
use tpt_nrg_wasm::{run_powerflow_json, validate_system_json};

let json = include_str!("../../../test-data/ieee/ieee14.json");
validate_system_json(json)?;

let result = run_powerflow_json(json)?;
// result.converged, result.iterations,
// result.voltage_magnitude_pu, result.voltage_angle_rad, result.losses_mw
```

The native build ships a thin shim that re-exports the underlying types; the `wasm32-unknown-unknown` target is needed to build the actual `wasm-bindgen` bindings for browser consumption.
