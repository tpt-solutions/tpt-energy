//! Substrate integration: validate the local distribution code paths against
//! `tpt-math-prob-dist`.
//!
//! The `substrate` feature exposes a sampling-based sanity check that draws
//! from the upstream `Normal` distribution and verifies the empirical
//! mean/variance match the analytical values within Monte-Carlo tolerance.

/// Sample `n` standard-normal draws using the upstream
/// `tpt-math-prob-dist` distribution and return `(mean, variance)`.
///
/// # Panics
///
/// Panics if the upstream normal-distribution constructor rejects the
/// standard-normal parameterization; it is validated, so this is
/// unreachable.
#[cfg(feature = "substrate")]
#[must_use]
pub fn normal_sample_mean_var(n: u64) -> (f64, f64) {
    use tpt_math_prob_core::{Distribution, SplitMix64};
    use tpt_math_prob_dist::{normal, Dist};

    let mut rng = SplitMix64::seed_from_u64(7);
    let dist = normal((0.0, 1.0)).expect("standard normal");
    let wrapper = Dist::new(dist);

    let mut sum = 0.0_f64;
    let mut sum_sq = 0.0_f64;
    // A draw count above 2^52 would lose precision in the f64 mean; such a
    // sample cannot be accumulated in memory anyway.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let n_f = n as f64;
    for _ in 0..n {
        let x: f64 = wrapper.sample(&mut rng);
        sum += x;
        sum_sq += x * x;
    }
    let mean = sum / n_f;
    let var = sum_sq / n_f - mean * mean;
    (mean, var)
}

#[cfg(all(test, feature = "substrate"))]
mod tests {
    use super::*;

    #[test]
    fn normal_sample_mean_var_match() {
        let (mean, var) = normal_sample_mean_var(100_000);
        // Standard normal: mean 0, variance 1; MC tolerance ~0.1 for 100k.
        assert!(mean.abs() < 0.05, "mean {mean}");
        assert!((var - 1.0).abs() < 0.05, "variance {var}");
    }
}
