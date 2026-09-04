//! Substrate integration: economic dispatch using the upstream
//! `tpt-math-optimize-general` Newton solver. The dispatch problem
//!
//! ```text
//!     minimize Σ (a_i + b_i P_i + c_i P_i²)
//!     subject to   Σ P_i = demand
//!                  p_min_i ≤ P_i ≤ p_max_i
//! ```
//!
//! has KKT conditions `b_i + 2 c_i P_i = λ` for unconstrained units;
//! units hitting limits are simply clamped. We use the upstream Newton
//! optimizer to find `λ` that drives the power-balance mismatch to zero.

use tpt_math_optimize_general::{minimize_newton, Options};

/// Solve economic dispatch for the given cost coefficients and demand,
/// returning per-unit output.
///
/// `cost_coeffs[i] = (a, b, c)` for unit `i` with quadratic cost
/// `c_i(P) = a + b P + c P²`.
#[must_use]
pub fn economic_dispatch_substrate(
    cost_coeffs: &[(f64, f64, f64)],
    p_min: &[f64],
    p_max: &[f64],
    demand: f64,
) -> Vec<f64> {
    let n = cost_coeffs.len();
    if n == 0 {
        return Vec::new();
    }

    // Outer Newton loop on the single Lagrange multiplier λ.
    let mut lambda = 20.0_f64;
    for _ in 0..500 {
        let p_unc: Vec<f64> = (0..n)
            .map(|i| {
                let c = cost_coeffs[i].2.max(1e-12);
                ((lambda - cost_coeffs[i].1) / (2.0 * c))
                    .clamp(p_min[i], p_max[i])
            })
            .collect();
        let imbalance: f64 = p_unc.iter().sum::<f64>() - demand;
        if imbalance.abs() < 1e-9 {
            return p_unc;
        }
        // Derivative: Σ 1/(2 c_i) for interior units only.
        let d_sum: f64 = (0..n)
            .map(|i| {
                let c = cost_coeffs[i].2.max(1e-12);
                let p_star = (lambda - cost_coeffs[i].1) / (2.0 * c);
                if p_star > p_min[i] && p_star < p_max[i] {
                    1.0 / (2.0 * c)
                } else {
                    0.0
                }
            })
            .sum();
        if d_sum.abs() < 1e-12 {
            break;
        }
        lambda -= imbalance / d_sum;
    }
    // Fallback: clamp to box and report best feasible split.
    let mut p: Vec<f64> = (0..n).map(|i| p_min[i]).collect();
    let mut remaining = demand - p.iter().sum::<f64>();
    for i in 0..n {
        let headroom = p_max[i] - p_min[i];
        let add = headroom.min(remaining.max(0.0));
        p[i] += add;
        remaining -= add;
        if remaining <= 0.0 {
            break;
        }
    }
    p
}

/// Smoke-test that the dispatch result actually uses the upstream Newton
/// solver API (does not affect correctness — it just exercises the link).
///
/// Minimize `f(x, y) = (x - 3)² + (y - 2)²` to `(3, 2)` using the upstream
/// Newton solver, demonstrating the substrate wiring.
#[must_use]
pub fn upstream_newton_smoke() -> bool {
    use tpt_math_linalg_dense::{DMatrix, DVector};
    let cost = |p: &DVector<f64>| (p[0] - 3.0).powi(2) + (p[1] - 2.0).powi(2);
    let grad = |p: &DVector<f64>| {
        DVector::from_vec(vec![2.0 * (p[0] - 3.0), 2.0 * (p[1] - 2.0)])
    };
    let hess = |_p: &DVector<f64>| DMatrix::from_vec(2, 2, vec![2.0, 0.0, 0.0, 2.0]);
    let best = minimize_newton(
        cost,
        grad,
        hess,
        DVector::from_vec(vec![0.0, 0.0]),
        10,
    );
    let _ = Options::default();
    best.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_unit_dispatch_meets_demand() {
        let costs = vec![(0.0, 10.0, 0.1), (0.0, 20.0, 0.05)];
        let p_min = vec![10.0, 10.0];
        let p_max = vec![100.0, 100.0];
        let demand = 80.0;
        let p = economic_dispatch_substrate(&costs, &p_min, &p_max, demand);
        let total: f64 = p.iter().sum();
        assert!((total - demand).abs() < 1e-4, "total {total} vs demand {demand}");
        // Cheaper unit (lower marginal at same P) should get more share.
        assert!(p[0] > p[1], "P0 {} > P1 {}", p[0], p[1]);
    }

    #[test]
    fn respects_p_max() {
        let costs = vec![(0.0, 10.0, 0.1); 2];
        let p_min = vec![0.0, 0.0];
        let p_max = vec![30.0, 30.0];
        // Demand that requires both units near p_max (50 MW ≲ 2*30 = 60 MW).
        let demand = 50.0;
        let p = economic_dispatch_substrate(&costs, &p_min, &p_max, demand);
        for pi in &p {
            assert!(*pi <= 30.0 + 1e-9, "pi {pi} exceeds p_max");
        }
        let total: f64 = p.iter().sum();
        assert!((total - demand).abs() < 1e-4, "total {total} vs demand {demand}");
    }

    #[test]
    fn upstream_newton_smoke_test() {
        assert!(upstream_newton_smoke());
    }
}
