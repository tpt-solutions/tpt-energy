# tpt-nrg-timeseries

Time-series containers and operations for load, generation, and price
profiles: uniformly-sampled series with resampling, and arbitrary
timestamped series with linear interpolation.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `UniformTimeSeries` — fixed-step series (e.g. hourly prices) with mean /
  min / max / energy integration, and step or linear resampling that
  validates step commensurability instead of silently relabeling the grid.
- `TimeSeries` — arbitrary `DateTime<Utc>`-stamped samples (e.g. SCADA)
  with sorted-order validation and linear interpolation that saturates
  outside the time range.
- Serde support on both containers.

## Installation

```toml
[dependencies]
tpt-nrg-timeseries = "0.1"
```

## Usage

```rust
use tpt_nrg_timeseries::{ResampleMethod, UniformTimeSeries};

let start = chrono::Utc::now();
let prices = UniformTimeSeries::new("price", "$/MWh", start, 3600,
    vec![20.0, 25.0, 40.0, 55.0, 30.0, 18.0]);

assert!((prices.mean() - 31.0).abs() < 1e-9);

// Downsample to 2-hour blocks by averaging.
let block_means = prices.resample(7200, ResampleMethod::Linear)?;
assert_eq!(block_means.len(), 3);
# Ok::<(), tpt_nrg_timeseries::TimeSeriesError>(())
```

Interpolating an irregular series:

```rust
use tpt_nrg_timeseries::TimeSeries;

let series = TimeSeries::new("load", "MW", samples); // Vec<(DateTime<Utc>, f64)>
let v = series.interpolate(timestamp); // linear between samples, saturating outside
```

## Crates.io metadata

- **Categories**: `science`, `data-structures`
- **Keywords**: `timeseries`, `resampling`, `interpolation`, `energy`, `profiles`

## Status

**Stable.** Unit tests cover integration, resampling (up/down,
commensurate and incommensurate), interpolation, and error cases.

## Testing

```sh
cargo test -p tpt-nrg-timeseries
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
