# `tpt-nrg-unit-commitment` · `tpt-nrg-economic-dispatch` · `tpt-nrg-reserve`

Dispatch and reserve crates.

## `tpt-nrg-unit-commitment`

Decide which thermal units to start up over a horizon to meet load at minimum cost.

The default implementation is a **priority-list / forward-dispatch** heuristic — units are sorted by average full-load cost and committed in merit order. This is fast, deterministic, and a recognised industry-grade approximation for planning studies.

```rust,no_run
use tpt_nrg_core::EnergySystem;
use tpt_nrg_unit_commitment::unit_commitment;

let system = EnergySystem::from_json(/* ... */)?;
let load_mw = vec![50.0, 75.0, 100.0, 125.0];
let r = unit_commitment(&system, &load_mw);
// r.commitment[u][t] == 1 if unit u is on at interval t
// r.outputs[u][t] in MW
// r.total_cost_dollar
```

Full **MILP** (min-up/down times, startup/shutdown costs, ramp constraints) is exposed via the `substrate` feature using `tpt-math-optimize-general`.

## `tpt-nrg-economic-dispatch`

Lossless merit-order dispatch with λ-iteration marginal pricing.

```rust,no_run
use tpt_nrg_core::EnergySystem;
use tpt_nrg_economic_dispatch::economic_dispatch;

let system = EnergySystem::from_json(/* ... */)?;
let r = economic_dispatch(&system, 200.0)?;
// r.generator_outputs_mw
// r.total_cost_dollar_per_h
// r.marginal_cost_dollar_per_mwh (λ)
// r.losses_mw
```

Plus a **storage arbitrage** helper:

```rust,no_run
use tpt_nrg_economic_dispatch::storage_arbitrage;

let plan = storage_arbitrage(
    &[10.0, 15.0, 20.0, 50.0, 80.0, 100.0], // prices $/MWh
    1.0,                                      // 1 h intervals
    100.0,                                    // 100 MWh capacity
    50.0,                                     // 50 MW power
    0.9,                                      // 90% round-trip
);
// plan.charge_mw, plan.discharge_mw, plan.state_of_charge, plan.net_revenue_dollar
```

## `tpt-nrg-reserve`

Spinning reserve margin and contingency (NERC-style N-1) requirement.

```rust,no_run
use tpt_nrg_reserve::*;

let spinning  = spinning_reserve_margin(200.0, 100.0, 80.0);    // MW headroom - load
let conting   = contingency_reserve_requirement(100.0, 500.0, 0.03); // N-1 + 3% load
let total     = total_operating_reserve(200.0, 100.0, 100.0, 500.0, 0.03);
```
