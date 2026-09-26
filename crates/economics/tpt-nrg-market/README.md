# tpt-nrg-market

Market-signal modeling: price statistics over time series for day-ahead,
real-time, and ancillary markets.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `MarketSignal` — wraps a `tpt-nrg-timeseries` price series with a
  `MarketType` (`DayAhead`, `RealTime`, `Ancillary`).
- `mean_price()` / `peak_price()` — summary statistics.
- `off_peak_price()` — 10th-percentile off-peak price, `None` for an
  empty series.
- `price_spread()` — peak minus off-peak; the spread is the arbitrage
  opportunity that `tpt-nrg-economic-dispatch::storage_arbitrage` can
  monetize.

## Installation

```toml
[dependencies]
tpt-nrg-market = "0.1"
```

## Usage

```rust
use chrono::TimeZone;
use tpt_nrg_market::{MarketSignal, MarketType};
use tpt_nrg_timeseries::UniformTimeSeries;

let ts = UniformTimeSeries::new(
    "da_price",
    "$/MWh",
    chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
    3600,
    vec![20.0, 18.0, 15.0, 60.0, 90.0, 75.0],
);
let sig = MarketSignal::new(MarketType::DayAhead, ts);

println!("mean {:.1}, peak {:.1}", sig.mean_price(), sig.peak_price());
if let Some(spread) = sig.price_spread() {
    println!("spread {spread:.1} $/MWh");
}
```

## Crates.io metadata

- **Categories**: `finance`, `science`
- **Keywords**: `electricity-market`, `price`, `day-ahead`, `energy`, `trading`

## Status

**Stable.** Unit-tested for statistics, positive spreads on realistic
shapes, and empty-series behavior.

## Testing

```sh
cargo test -p tpt-nrg-market
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
