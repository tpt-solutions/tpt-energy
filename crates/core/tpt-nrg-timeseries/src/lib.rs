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
    InvalidStep(i64),
    /// The requested resample step is not an integer multiple (or divisor)
    /// of the current step, so the sample grid cannot be mapped exactly.
    #[error("resample step {0} s is not an integer multiple or divisor of the current step")]
    IncommensurateStep(i64),
}

/// Result alias for `tpt-nrg-timeseries`.
pub type TimeSeriesResult<T> = Result<T, TimeSeriesError>;

/// Numeric conversions between sample counts / durations and `f64`.
///
/// Conversions are exact for magnitudes up to 2^52, far beyond any
/// realistic step length or sample count; the precision-loss allowances
/// live at this single documented point instead of scattering `as f64`
/// through the numeric code.
pub(crate) mod conv {
    /// Convert a duration in seconds/milliseconds to `f64`.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn i64_to_f64(value: i64) -> f64 {
        value as f64
    }

    /// Convert a sample count to `f64`.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn usize_to_f64(value: usize) -> f64 {
        value as f64
    }
}
