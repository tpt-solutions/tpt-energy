# tpt-nrg-state-estimation

Grid state estimation on the DC approximation: a B-matrix-based
weighted-least-squares solver over a sparse set of noisy measurements,
with an optional FIR measurement-noise filter.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `run_dc_state_estimation()` — WLS estimate of bus voltage angles from
  `PowerInjection` / `BranchFlow` measurements with per-measurement
  variance.
- `Measurement` enum covering active-power injections and branch flows.
- Measurement-noise low-pass filtering behind the `substrate` feature
  (`smooth_voltage_measurements` via `tpt-math-signal-filter`).
- Suited to small-system validation and unit tests; the full non-linear
  WLS estimator with bad-data detection is future work (see the crate
  status page in the book).

## Installation

```toml
[dependencies]
tpt-nrg-state-estimation = "0.1"
```

## Usage

```rust
use tpt_nrg_state_estimation::{run_dc_state_estimation, Measurement};

let system = tpt_nrg_core::EnergySystem::from_json_file("ieee14.json")?;
let measurements = vec![
    Measurement::PowerInjection { bus: 2, value_mw: -21.7, variance: 0.1 },
];
let result = run_dc_state_estimation(&system, &measurements);
println!("estimated angles: {:?}", result.bus_voltage_angle_rad);
```

## Crates.io metadata

- **Categories**: `science`, `algorithms`
- **Keywords**: `state-estimation`, `power-systems`, `scada`, `filtering`, `measurements`

## Status

**Alpha.** The DC estimator is complete and tested; the full non-linear
WLS estimator with IEEE 118-bus validation is planned.

## Testing

```sh
cargo test -p tpt-nrg-state-estimation --all-features
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
