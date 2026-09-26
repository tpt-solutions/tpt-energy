# tpt-nrg-der

Distributed energy resource (DER) asset models and microgrid controller
state.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `DerAsset` — solar, wind, battery, load, and diesel genset variants
  with per-asset `net_power_mw()` (signed supply/demand contribution).
- `MicrogridController` — grid-connected flag, asset list, control
  strategy, and load tracking with generation / demand / net-balance
  aggregation.
- `ControlStrategy` — `GridFollowing`, `GridForming`, `DroopControl`.
- Serde-friendly enums for configuration-file driven studies.

## Installation

```toml
[dependencies]
tpt-nrg-der = "0.1"
```

## Usage

```rust
use tpt_nrg_der::{ControlStrategy, DerAsset, MicrogridController};

let mut controller = MicrogridController::new(ControlStrategy::GridForming, 6.0);
controller.add_asset(DerAsset::Solar { capacity_mw: 10.0, output_fraction: 0.5 });
controller.add_asset(DerAsset::Diesel {
    rated_mw: 4.0,
    output_mw: 3.0,
    min_output_mw: 1.0,
});

println!("generation {} MW, demand {} MW, net {} MW",
    controller.total_generation_mw(),
    controller.total_demand_mw(),
    controller.net_balance_mw());
```

## Crates.io metadata

- **Categories**: `science`, `simulation`
- **Keywords**: `der`, `microgrid`, `grid-forming`, `inverter`, `distributed-energy`

## Status

**Stable.** Unit-tested for asset power accounting and controller
balance.

## Testing

```sh
cargo test -p tpt-nrg-der
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
