//! # tpt-nrg-reserve
//!
//! Spinning and contingency reserve calculations.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// Reserve requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReserveRequirement {
    /// Spinning reserve required in MW.
    pub spinning_mw: f64,
    /// Contingency (non-spinning) reserve required in MW.
    pub contingency_mw: f64,
}

/// Spinning-reserve margin: `(available headroom on online units) - load`.
pub fn spinning_reserve_margin(
    online_capacity_mw: f64,
    current_output_mw: f64,
    load_mw: f64,
) -> f64 {
    let headroom = (online_capacity_mw - current_output_mw).max(0.0);
    headroom - load_mw
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

/// Total operating reserve requirement: spinning + contingency.
pub fn total_operating_reserve(
    online_capacity_mw: f64,
    current_output_mw: f64,
    largest_online_unit_mw: f64,
    load_mw: f64,
    load_fraction: f64,
) -> ReserveRequirement {
    let spinning = spinning_reserve_margin(online_capacity_mw, current_output_mw, load_mw);
    let contingency = contingency_reserve_requirement(largest_online_unit_mw, load_mw, load_fraction);
    ReserveRequirement {
        spinning_mw: spinning,
        contingency_mw: contingency,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinning_margin_positive_with_headroom() {
        let m = spinning_reserve_margin(200.0, 100.0, 80.0);
        // Headroom 100, minus load 80 = 20
        assert!((m - 20.0).abs() < 1e-9);
    }

    #[test]
    fn spinning_margin_negative_when_loaded() {
        let m = spinning_reserve_margin(200.0, 150.0, 180.0);
        // Headroom 50, minus load 180 = -130 (deficit)
        assert!((m - (-130.0)).abs() < 1e-9);
    }

    #[test]
    fn contingency_n_minus_1() {
        // Largest unit 100 MW, load 500 MW, fraction 3%
        let c = contingency_reserve_requirement(100.0, 500.0, 0.03);
        assert!((c - 115.0).abs() < 1e-9);
    }
}
