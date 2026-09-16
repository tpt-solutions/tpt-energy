//! Uniformly-sampled time series.

use serde::{Deserialize, Serialize};

use crate::time_series::TimeSeries;
use crate::{TimeSeriesError, TimeSeriesResult};

/// Resampling method for [`UniformTimeSeries`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResampleMethod {
    /// Step-and-hold: each new sample takes the previous value.
    Step,
    /// Linear interpolation between neighboring samples.
    Linear,
}

/// A regularly-sampled time series with a fixed step duration.
///
/// `start` is the timestamp of the first sample; subsequent samples are spaced
/// by `step_seconds` seconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniformTimeSeries {
    /// ISO-8601 timestamp of the first sample.
    pub start: chrono::DateTime<chrono::Utc>,
    /// Step duration in seconds.
    pub step_seconds: i64,
    /// Sample values (length = `n`).
    pub values: Vec<f64>,
    /// Human-readable name (e.g. "load_mw").
    pub name: String,
    /// Units string (e.g. "MW", "$/MWh").
    pub units: String,
}

impl UniformTimeSeries {
    /// Construct a new uniform time series.
    pub fn new(
        name: impl Into<String>,
        units: impl Into<String>,
        start: chrono::DateTime<chrono::Utc>,
        step_seconds: i64,
        values: Vec<f64>,
    ) -> Self {
        Self {
            name: name.into(),
            units: units.into(),
            start,
            step_seconds,
            values,
        }
    }

    /// Number of samples.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the series has no samples.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Get the value at index `i`.
    #[must_use]
    pub fn get(&self, i: usize) -> Option<f64> {
        self.values.get(i).copied()
    }

    /// Total energy integrated over the series (assumes values are in MW or
    /// equivalent power units and step is in seconds).
    #[must_use]
    pub fn integrate_mwh(&self) -> f64 {
        // sum(MW) * step(h) = MWh
        let step_h = self.step_seconds as f64 / 3600.0;
        self.values.iter().sum::<f64>() * step_h
    }

    /// Mean value.
    #[must_use]
    pub fn mean(&self) -> f64 {
        if self.values.is_empty() {
            0.0
        } else {
            self.values.iter().sum::<f64>() / self.values.len() as f64
        }
    }

    /// Maximum value.
    #[must_use]
    pub fn max(&self) -> f64 {
        self.values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
    }

    /// Minimum value.
    #[must_use]
    pub fn min(&self) -> f64 {
        self.values.iter().copied().fold(f64::INFINITY, f64::min)
    }

    /// Resample to a new step duration.
    ///
    /// `new_step_seconds` must be a positive integer multiple (downsampling)
    /// or divisor (upsampling) of the current step. Decimation is performed
    /// by averaging; upsampling uses the requested [`ResampleMethod`].
    pub fn resample(
        &self,
        new_step_seconds: i64,
        method: ResampleMethod,
    ) -> TimeSeriesResult<UniformTimeSeries> {
        if new_step_seconds <= 0 {
            return Err(TimeSeriesError::InvalidStep(new_step_seconds as f64));
        }
        if self.step_seconds <= 0 {
            return Err(TimeSeriesError::InvalidStep(self.step_seconds as f64));
        }
        if self.values.is_empty() {
            return Err(TimeSeriesError::Empty);
        }
        if new_step_seconds >= self.step_seconds {
            if new_step_seconds % self.step_seconds != 0 {
                return Err(TimeSeriesError::IncommensurateStep(new_step_seconds as f64));
            }
            // Downsample: average
            let factor = (new_step_seconds / self.step_seconds) as usize;
            let mut values = Vec::new();
            for chunk in self.values.chunks(factor) {
                let avg = chunk.iter().sum::<f64>() / chunk.len() as f64;
                values.push(avg);
            }
            Ok(UniformTimeSeries {
                name: self.name.clone(),
                units: self.units.clone(),
                start: self.start,
                step_seconds: new_step_seconds,
                values,
            })
        } else {
            if self.step_seconds % new_step_seconds != 0 {
                return Err(TimeSeriesError::IncommensurateStep(new_step_seconds as f64));
            }
            // Upsample via interpolation
            let factor = (self.step_seconds / new_step_seconds) as usize;
            let mut values = Vec::with_capacity((self.values.len() - 1) * factor + 1);
            for w in self.values.windows(2) {
                let (v0, v1) = (w[0], w[1]);
                values.push(v0);
                for k in 1..factor {
                    let t = k as f64 / factor as f64;
                    let v = match method {
                        ResampleMethod::Step => v0,
                        ResampleMethod::Linear => v0 + t * (v1 - v0),
                    };
                    values.push(v);
                }
            }
            values.push(*self.values.last().unwrap());
            Ok(UniformTimeSeries {
                name: self.name.clone(),
                units: self.units.clone(),
                start: self.start,
                step_seconds: new_step_seconds,
                values,
            })
        }
    }
}

impl From<UniformTimeSeries> for TimeSeries {
    fn from(u: UniformTimeSeries) -> Self {
        let samples: Vec<(chrono::DateTime<chrono::Utc>, f64)> = u
            .values
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let t = u.start + chrono::Duration::seconds(i as i64 * u.step_seconds);
                (t, v)
            })
            .collect();
        TimeSeries {
            name: u.name,
            units: u.units,
            samples,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn series() -> UniformTimeSeries {
        let start = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        UniformTimeSeries::new(
            "load_mw",
            "MW",
            start,
            3600,
            vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0],
        )
    }

    #[test]
    fn integrate_mwh_hourly() {
        let s = series();
        // Sum = 210 MW for 6 hours -> 210 MWh
        assert!((s.integrate_mwh() - 210.0).abs() < 1e-9);
    }

    #[test]
    fn mean_and_extremes() {
        let s = series();
        assert!((s.mean() - 35.0).abs() < 1e-9);
        assert!((s.min() - 10.0).abs() < 1e-9);
        assert!((s.max() - 60.0).abs() < 1e-9);
    }

    #[test]
    fn downsample_by_averaging() {
        let s = series();
        let r = s.resample(7200, ResampleMethod::Linear).unwrap();
        assert_eq!(r.step_seconds, 7200);
        assert_eq!(r.values.len(), 3);
        // (10+20)/2 = 15, (30+40)/2 = 35, (50+60)/2 = 55
        assert!((r.values[0] - 15.0).abs() < 1e-9);
        assert!((r.values[1] - 35.0).abs() < 1e-9);
        assert!((r.values[2] - 55.0).abs() < 1e-9);
    }

    #[test]
    fn upsample_linear() {
        let s = series();
        let r = s.resample(1800, ResampleMethod::Linear).unwrap();
        assert_eq!(r.step_seconds, 1800);
        // 6 samples at 1h -> 11 samples at 0.5h
        assert_eq!(r.values.len(), 11);
        assert!((r.values[0] - 10.0).abs() < 1e-9);
        assert!((r.values[1] - 15.0).abs() < 1e-9);
        assert!((r.values[2] - 20.0).abs() < 1e-9);
        assert!((r.values[10] - 60.0).abs() < 1e-9);
    }

    #[test]
    fn invalid_step() {
        let s = series();
        let r = s.resample(0, ResampleMethod::Linear);
        assert!(r.is_err());
    }

    #[test]
    fn resample_rejects_empty_series() {
        let start = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let s = UniformTimeSeries::new("x", "MW", start, 3600, vec![]);
        assert!(matches!(
            s.resample(7200, ResampleMethod::Linear),
            Err(TimeSeriesError::Empty)
        ));
        assert!(matches!(
            s.resample(1800, ResampleMethod::Linear),
            Err(TimeSeriesError::Empty)
        ));
    }

    #[test]
    fn resample_rejects_incommensurate_steps() {
        let s = series();
        // 5400 s is neither a multiple of 3600 s nor a divisor of it; the
        // old behavior silently kept the values and relabeled the step.
        assert!(matches!(
            s.resample(5400, ResampleMethod::Linear),
            Err(TimeSeriesError::IncommensurateStep(_))
        ));
        assert!(s.resample(1000, ResampleMethod::Step).is_err());
    }

    #[test]
    fn resample_single_value_upsample() {
        let start = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let s = UniformTimeSeries::new("x", "MW", start, 3600, vec![42.0]);
        let r = s.resample(1800, ResampleMethod::Linear).unwrap();
        assert_eq!(r.values, vec![42.0]);
    }
}
