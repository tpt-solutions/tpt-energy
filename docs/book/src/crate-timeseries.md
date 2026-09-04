# `tpt-nrg-timeseries`

Container types for load, generation, and price profiles.

## Uniform time-series

For regularly-sampled data (most common case):

```rust,no_run
use tpt_nrg_timeseries::UniformTimeSeries;
use chrono::{TimeZone, Utc};

let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
let values: Vec<f64> = (0..24).map(|h| 50.0 + 20.0 * (h as f64 * std::f64::consts::PI / 12.0).sin()).collect();
let ts = UniformTimeSeries::new("price", "$/MWh", start, 3600, values);

// Helpers
let mean = ts.mean();
let peak = ts.max();
let total = ts.integrate_trapezoid();
```

## Irregular time-series

For measurements at irregular intervals (SCADA, AMI, etc.):

```rust,no_run
use tpt_nrg_timeseries::TimeSeries;
use chrono::{TimeZone, Utc};

let ts = TimeSeries::new("load", "MW", vec![
    (Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),  50.0),
    (Utc.with_ymd_and_hms(2026, 1, 1, 1, 0, 0).unwrap(),  48.0),
    (Utc.with_ymd_and_hms(2026, 1, 1, 3, 0, 0).unwrap(),  55.0),  // missing 2am sample
]);

let at_2am = ts.interpolate(Utc.with_ymd_and_hms(2026, 1, 1, 2, 0, 0).unwrap());
```

## Resampling

```rust,no_run
# use tpt_nrg_timeseries::UniformTimeSeries;
# use chrono::{TimeZone, Utc};
# let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
# let hourly = UniformTimeSeries::new("p", "MW", start, 3600, vec![1.0; 24]);
let quarter_hour = hourly.upsample_linear(900);   // 24 -> 96 samples
let six_hourly   = hourly.downsample_by_average(6); // 24 -> 4 samples
```
