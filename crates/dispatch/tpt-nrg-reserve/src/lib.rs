//! # tpt-nrg-reserve
//!
//! Spinning and contingency reserve calculations.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// Reserve adequacy assessment for a system snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReserveAssessment {
    /// Available spinning reserve in MW (headroom on online units).
    pub spinning_available_mw: f64,
    /// Contingency reserve required in MW (NERC-style N-1 + load fraction).
    pub contingency_required_mw: f64,
    /// `spinning_available_mw − contingency_required_mw`: negative means a
    /// reserve deficit.
    pub margin_mw: f64,
}

/// Available spinning reserve in MW: headroom on online units
/// (`online_capacity − current_output`, floored at 0).
///
/// This is pure availability — it does *not* subtract load, because headroom
/// already nets out the load being served. Compare it against a requirement
/// (e.g. [`contingency_reserve_requirement`]) to judge adequacy; see
/// [`assess_reserves`].
pub fn spinning_reserve_margin(online_capacity_mw: f64, current_output_mw: f64) -> f64 {
    (online_capacity_mw - current_output_mw).max(0.0)
}

/// Contingency-reserve requirement as a fraction of the largest online
/// unit, plus a fraction of load (NERC-style: N-1 + 3% of load).
pub fn contingency_reserve_requirement(
    largest_online_unit_mw: f64,
    load_mw: f64,
    load_fraction: f64,
) -> f64 {
    largest_online_unit_mw + load_fraction * load_mw
}

/// Assess spinning reserve availability against the contingency-reserve
/// requirement for a system snapshot.
pub fn assess_reserves(
    online_capacity_mw: f64,
    current_output_mw: f64,
    largest_online_unit_mw: f64,
    load_mw: f64,
    load_fraction: f64,
) -> ReserveAssessment {
    let spinning_available_mw = spinning_reserve_margin(online_capacity_mw, current_output_mw);
    let contingency_required_mw =
        contingency_reserve_requirement(largest_online_unit_mw, load_mw, load_fraction);
    ReserveAssessment {
        spinning_available_mw,
        contingency_required_mw,
        margin_mw: spinning_available_mw - contingency_required_mw,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinning_margin_is_headroom() {
        // 200 MW online, 100 MW output → 100 MW of spinning reserve.
        assert!((spinning_reserve_margin(200.0, 100.0) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn spinning_margin_clamped_non_negative() {
        assert_eq!(spinning_reserve_margin(200.0, 250.0), 0.0);
    }

    #[test]
    fn contingency_n_minus_1() {
        // Largest unit 100 MW, load 500 MW, fraction 3%
        let c = contingency_reserve_requirement(100.0, 500.0, 0.03);
        assert!((c - 115.0).abs() < 1e-9);
    }

    #[test]
    fn assessment_flags_deficit() {
        // Headroom 50 MW vs a 115 MW requirement → 65 MW deficit.
        let a = assess_reserves(200.0, 150.0, 100.0, 500.0, 0.03);
        assert!((a.spinning_available_mw - 50.0).abs() < 1e-9);
        assert!((a.contingency_required_mw - 115.0).abs() < 1e-9);
        assert!((a.margin_mw - (-65.0)).abs() < 1e-9);
    }
}
