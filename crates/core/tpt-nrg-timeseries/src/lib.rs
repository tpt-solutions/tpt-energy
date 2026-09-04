//! # tpt-nrg-timeseries
//!
//! Time-series containers and basic operations for load, generation, and price
//! profiles.
//!
//! Two main types are provided:
//!
//! - [`UniformTimeSeries`]: regularly-spaced samples keyed by a `step`
//!   duration (e.g. hourly).
//! - [`TimeSeries`]: arbitrary time-stamped samples (e.g. from SCADA).

#![deny(missing_docs)]

mod time_series;
mod uniform;

pub use time_series::TimeSeries;
pub use uniform::{ResampleMethod, UniformTimeSeries};

use thiserror::Error;

/// Errors for time-series operations.
#[derive(Debug, Error)]
pub enum TimeSeriesError {
    /// The series has no samples.
    #[error("time series is empty")]
    Empty,
    /// Samples are not sorted in ascending time order.
    #[error("time series samples must be sorted by timestamp in ascending order")]
    NotSorted,
    /// The requested resample step is non-positive.
    #[error("resample step must be > 0 (got {0})")]
    InvalidStep(f64),
}

/// Result alias for `tpt-nrg-timeseries`.
pub type TimeSeriesResult<T> = Result<T, TimeSeriesError>;
