# tpt-nrg-vpp

Virtual power plant: aggregation of DER fleets for market
participation, with flexible-capacity accounting and pro-rata dispatch.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `VirtualPowerPlant` — per-asset capacity / output / up-flex / down-flex
  vectors.
- `AggregationModel` — plain `Sum` or `DiversityAdjusted` with a
  diversity factor in (0, 1].
- `flexible_capacity_up_mw()` / `flexible_capacity_down_mw()` — MW
  available for demand response.
- `dispatch_assets()` — pro-rata `DispatchPlan` with delivered and
  unfulfilled components when the request exceeds available flexibility.

## Installation

```toml
[dependencies]
tpt-nrg-vpp = "0.1"
```

## Usage

```rust
use tpt_nrg_vpp::{AggregationModel, VirtualPowerPlant};

// (capacity, output, up-flex, down-flex) per asset, MW.
let vpp = VirtualPowerPlant::new(
    vec![(10.0, 5.0, 2.0, 2.0), (20.0, 10.0, 5.0, 3.0)],
    AggregationModel::Sum,
);

assert!((vpp.flexible_capacity_up_mw() - 7.0).abs() < 1e-9);
let plan = vpp.dispatch_assets(3.5);
println!("delivered {:.1} MW of 3.5 MW", plan.delivered_mw);
```

## Crates.io metadata

- **Categories**: `science`, `simulation`
- **Keywords**: `vpp`, `virtual-power-plant`, `aggregation`, `demand-response`, `der`

## Status

**Stable.** Unit-tested for capacity sums, diversity adjustment, pro-rata
dispatch, and flex capping.

## Testing

```sh
cargo test -p tpt-nrg-vpp
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
