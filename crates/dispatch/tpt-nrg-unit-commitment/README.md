# tpt-nrg-unit-commitment

Unit commitment: decide which units to start up over a horizon to meet
demand at minimum cost, respecting unit limits and merit order.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- Priority-list / forward-dispatch heuristic: units sorted by average
  full-load cost, committed in merit order per interval.
- `UnitCommitmentResult` — per-unit per-interval commitment matrix,
  outputs (MW), and horizon cost.
- Golden fixture (`test-data/golden/dispatch/unit-commitment-24hr.json`)
  verifying the commitment serves a 24-hour load profile.
- `substrate` feature — an upstream-validated continuous-dispatch solver
  (`economic_dispatch_substrate`) built on `tpt-math-optimize-general`;
  the full MILP (min-up/down times, startup costs, ramps) lands when the
  substrate MILP solver ships.

## Installation

```toml
[dependencies]
tpt-nrg-unit-commitment = "0.1"
```

## Usage

```rust
use tpt_nrg_core::EnergySystem;
use tpt_nrg_unit_commitment::unit_commitment;

let system = EnergySystem::from_json_file("system.json")?;
let load_mw = vec![50.0, 75.0, 100.0, 125.0];
let r = unit_commitment(&system, &load_mw);

// r.commitment[u][t] == 1 if unit u is on at interval t
// r.outputs[u][t] in MW, r.total_cost_dollar over the horizon
println!("cost ${:.0}", r.total_cost_dollar);
# Ok::<(), tpt_nrg_core::CoreError>(())
```

## Crates.io metadata

- **Categories**: `science`, `algorithms`
- **Keywords**: `unit-commitment`, `dispatch`, `scheduling`, `power-systems`, `merit-order`

## Status

**Alpha.** The heuristic is deterministic and tested, but the API may
grow as the full MILP formulation arrives behind the `substrate` feature.

## Testing

```sh
cargo test -p tpt-nrg-unit-commitment --all-features
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
