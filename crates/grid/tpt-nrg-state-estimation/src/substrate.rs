//! Substrate integration: use `tpt-math-signal-filter` to apply an FIR
//! low-pass to bus-voltage measurements before they enter the Kalman
//! filter, reducing measurement noise variance.

#[cfg(feature = "substrate")]
use tpt_math_signal_filter::FirFilter;

/// Build an FIR low-pass filter with `n_taps` and cutoff (cycles/sample).
#[cfg(feature = "substrate")]
#[must_use]
pub fn measurement_lowpass(n_taps: usize, cutoff: f64) -> FirFilter {
    FirFilter::lowpass(n_taps, cutoff)
}

/// Smooth a sequence of bus voltage magnitudes using a windowed-sinc FIR
/// low-pass filter.
#[cfg(feature = "substrate")]
#[must_use]
pub fn smooth_voltage_measurements(measurements: &[f64], n_taps: usize, cutoff: f64) -> Vec<f64> {
    let mut buf = measurements.to_vec();
    measurement_lowpass(n_taps, cutoff).filter_in_place(&mut buf);
    buf
}

#[cfg(all(test, feature = "substrate"))]
mod tests {
    use super::*;

    #[test]
    fn lowpass_preserves_dc() {
        let n_taps = 15;
        let cutoff = 0.1;
        let measurement = vec![1.05; 64];
        let smoothed = smooth_voltage_measurements(&measurement, n_taps, cutoff);
        // Interior samples (away from edge transients) should equal DC.
        for (i, &v) in smoothed.iter().enumerate().take(40).skip(8) {
            assert!((v - 1.05).abs() < 1e-6, "sample {i}: smoothed {v} != 1.05");
        }
    }

    #[test]
    fn lowpass_attenuates_high_freq() {
        // 64 samples of a sine at near-Nyquist frequency.
        let n = 64;
        let signal: Vec<f64> = (0..n)
            .map(|i| (std::f64::consts::PI * i as f64 * 0.49).sin())
            .collect();
        let smoothed = smooth_voltage_measurements(&signal, 15, 0.1);
        // Interior RMS of smoothed should be much smaller than original.
        let orig_rms: f64 = (signal.iter().map(|x| x * x).sum::<f64>() / n as f64).sqrt();
        let sm_rms: f64 =
            (smoothed.iter().skip(8).take(48).map(|x| x * x).sum::<f64>() / 48.0).sqrt();
        assert!(sm_rms < 0.3 * orig_rms, "sm {sm_rms} vs orig {orig_rms}");
    }
}
