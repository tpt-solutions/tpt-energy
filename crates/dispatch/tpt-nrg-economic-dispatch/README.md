# tpt-nrg-economic-dispatch

Economic dispatch and storage arbitrage: merit-order dispatch with
lambda-iteration marginal pricing, plus a price-forecast storage
scheduler.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `economic_dispatch()` — lossless dispatch finding the system lambda
  that balances load, respecting per-unit `p_min`/`p_max` and treating
  curve-less units as must-run at `p_min`.
- `EconomicDispatchResult` — per-generator outputs, total cost, marginal
  cost (lambda), and losses (zero for the lossless model).
- `storage_arbitrage()` — charge in the cheapest third of the price
  forecast, discharge in the most expensive third, respecting energy and
  power limits and round-trip efficiency; returns an `ArbitragePlan`.
- Typed errors for the infeasible cases (no generators, load above
  capacity, load below must-run minimum).
- Golden fixture (`test-data/golden/dispatch/economic-dispatch-5gen.json`).

## Installation

```toml
[dependencies]
tpt-nrg-economic-dispatch = "0.1"
```

## Usage

```rust
use tpt_nrg_core::EnergySystem;
use tpt_nrg_economic_dispatch::{economic_dispatch, storage_arbitrage};

let system = EnergySystem::from_json_file("system.json")?;
let r = economic_dispatch(&system, 200.0)?;
println!("lambda = {:.2} $/MWh", r.marginal_cost_dollar_per_mwh);

let plan = storage_arbitrage(
    &[10.0, 15.0, 20.0, 50.0, 80.0, 100.0], // prices $/MWh
    1.0,   // interval length, h
    100.0, // energy capacity, MWh
    50.0,  // power rating, MW
    0.9,   // round-trip efficiency
);
println!("arbitrage revenue ${:.0}", plan.net_revenue_dollar);
# Ok::<(), tpt_nrg_economic_dispatch::DispatchError>(())
```

## Crates.io metadata

- **Categories**: `science`, `algorithms`
- **Keywords**: `economic-dispatch`, `dispatch`, `merit-order`, `arbitrage`, `power-systems`

## Status

**Stable.** Golden-tested against the 5-generator fixture; unit tests
verify lambda equals the marginal unit's incremental cost and that
infeasible loads raise typed errors.

## Testing

```sh
cargo test -p tpt-nrg-economic-dispatch
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
