//! Arbitrary-timestamp time series.

use serde::{Deserialize, Serialize};

use crate::{TimeSeriesError, TimeSeriesResult};

/// A time-stamped series of scalar values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries {
    /// Human-readable name.
    pub name: String,
    /// Units string.
    pub units: String,
    /// Samples in ascending timestamp order.
    pub samples: Vec<(chrono::DateTime<chrono::Utc>, f64)>,
}

impl TimeSeries {
    /// Construct a new series.
    pub fn new(
        name: impl Into<String>,
        units: impl Into<String>,
        samples: Vec<(chrono::DateTime<chrono::Utc>, f64)>,
    ) -> Self {
        Self {
            name: name.into(),
            units: units.into(),
            samples,
        }
    }

    /// Number of samples.
    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether the series is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Verify that samples are in ascending timestamp order.
    pub fn validate_sorted(&self) -> TimeSeriesResult<()> {
        for w in self.samples.windows(2) {
            if w[0].0 > w[1].0 {
                return Err(TimeSeriesError::NotSorted);
            }
        }
        Ok(())
    }

    /// Linearly interpolate (or step-hold) the value at time `t`.
    ///
    /// Returns `None` if the series is empty. Saturates at the first/last
    /// sample outside the time range.
    #[must_use]
    pub fn interpolate(&self, t: chrono::DateTime<chrono::Utc>) -> Option<f64> {
        if self.samples.is_empty() {
            return None;
        }
        if t <= self.samples[0].0 {
            return Some(self.samples[0].1);
        }
        if let Some(last) = self.samples.last() {
            if t >= last.0 {
                return Some(last.1);
            }
        }
        for w in self.samples.windows(2) {
            let (t0, v0) = w[0];
            let (t1, v1) = w[1];
            if t >= t0 && t <= t1 {
                let span = (t1 - t0).num_milliseconds() as f64;
                if span == 0.0 {
                    return Some(v0);
                }
                let frac = (t - t0).num_milliseconds() as f64 / span;
                return Some(v0 + frac * (v1 - v0));
            }
        }
        None
    }

    /// Mean value.
    #[must_use]
    pub fn mean(&self) -> f64 {
        if self.samples.is_empty() {
            0.0
        } else {
            self.samples.iter().map(|(_, v)| *v).sum::<f64>() / self.samples.len() as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn series() -> TimeSeries {
        let t0 = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        TimeSeries::new(
            "load",
            "MW",
            vec![
                (t0, 10.0),
                (t0 + chrono::Duration::hours(1), 20.0),
                (t0 + chrono::Duration::hours(2), 30.0),
            ],
        )
    }

    #[test]
    fn validate_sorted_ok() {
        series().validate_sorted().unwrap();
    }

    #[test]
    fn validate_sorted_err() {
        let t0 = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let s = TimeSeries::new(
            "x",
            "x",
            vec![(t0 + chrono::Duration::hours(1), 1.0), (t0, 2.0)],
        );
        assert!(s.validate_sorted().is_err());
    }

    #[test]
    fn interpolate_inside() {
        let s = series();
        let t = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 30, 0).unwrap();
        assert!((s.interpolate(t).unwrap() - 15.0).abs() < 1e-9);
    }

    #[test]
    fn interpolate_outside_saturates() {
        let s = series();
        let t_before = chrono::Utc.with_ymd_and_hms(2025, 12, 31, 0, 0, 0).unwrap();
        assert!((s.interpolate(t_before).unwrap() - 10.0).abs() < 1e-9);
        let t_after = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 5, 0, 0).unwrap();
        assert!((s.interpolate(t_after).unwrap() - 30.0).abs() < 1e-9);
    }

    #[test]
    fn mean_value() {
        let s = series();
        assert!((s.mean() - 20.0).abs() < 1e-9);
    }
}
