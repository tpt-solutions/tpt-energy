//! Branch (transmission line / transformer) definition.

use serde::{Deserialize, Serialize};

/// A transmission branch connecting two buses.
///
/// All impedance values are in per-unit on the system base. A tap ratio of
/// `1.0` and a phase shift of `0.0` represent an unscaled line; transformers
/// typically use non-unity tap ratios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    /// Unique numeric branch identifier.
    pub id: usize,

    /// Human-readable branch name.
    pub name: String,

    /// "from" bus id (sending end).
    pub from_bus: usize,

    /// "to" bus id (receiving end).
    pub to_bus: usize,

    /// Series resistance in per-unit.
    pub resistance_pu: f64,

    /// Series reactance in per-unit.
    pub reactance_pu: f64,

    /// Total charging susceptance in per-unit (B/2 at each end summed).
    #[serde(default)]
    pub susceptance_pu: f64,

    /// Off-nominal turns ratio (1.0 = no transformer).
    #[serde(default = "default_tap")]
    pub tap_ratio: f64,

    /// Phase-shifting angle in radians.
    #[serde(default)]
    pub phase_shift_rad: f64,

    /// Long-term thermal rating in MVA.
    #[serde(default = "default_rating")]
    pub rating_mva: f64,

    /// Whether the branch is currently in service.
    #[serde(default = "default_in_service")]
    pub in_service: bool,
}

fn default_tap() -> f64 {
    1.0
}
fn default_rating() -> f64 {
    100.0
}
fn default_in_service() -> bool {
    true
}

impl Branch {
    /// Construct a new branch. Use [`with_tap`](Self::with_tap) to model a
    /// transformer and [`with_rating`](Self::with_rating) to set the thermal
    /// limit.
    pub fn new(
        id: usize,
        name: impl Into<String>,
        from_bus: usize,
        to_bus: usize,
        resistance_pu: f64,
        reactance_pu: f64,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            from_bus,
            to_bus,
            resistance_pu,
            reactance_pu,
            susceptance_pu: 0.0,
            tap_ratio: 1.0,
            phase_shift_rad: 0.0,
            rating_mva: default_rating(),
            in_service: true,
        }
    }

    /// Set the total line charging susceptance in per-unit.
    pub fn with_susceptance(mut self, b_pu: f64) -> Self {
        self.susceptance_pu = b_pu;
        self
    }

    /// Set the transformer tap ratio and phase shift.
    pub fn with_tap(mut self, tap_ratio: f64, phase_shift_rad: f64) -> Self {
        self.tap_ratio = tap_ratio;
        self.phase_shift_rad = phase_shift_rad;
        self
    }

    /// Set the long-term thermal rating in MVA.
    pub fn with_rating(mut self, rating_mva: f64) -> Self {
        self.rating_mva = rating_mva;
        self
    }

    /// Mark the branch as in or out of service.
    pub fn with_in_service(mut self, in_service: bool) -> Self {
        self.in_service = in_service;
        self
    }

    /// Compute the series admittance `y = 1 / (r + jx)` of the branch.
    #[must_use]
    pub fn series_admittance(&self) -> (f64, f64) {
        let z_re = self.resistance_pu;
        let z_im = self.reactance_pu;
        let denom = z_re * z_re + z_im * z_im;
        if denom == 0.0 {
            (0.0, 0.0)
        } else {
            (z_re / denom, -z_im / denom)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_series_admittance() {
        let b = Branch::new(1, "L1", 1, 2, 0.01, 0.1);
        let (g, bi) = b.series_admittance();
        // y = 1 / (0.01 + j0.1) = (0.01 - j0.1) / 0.0101
        let g_expected = 0.01 / 0.0101;
        let b_expected = -0.1 / 0.0101;
        assert!((g - g_expected).abs() < 1e-9);
        assert!((bi - b_expected).abs() < 1e-9);
    }

    #[test]
    fn branch_with_tap() {
        let b = Branch::new(1, "T1", 1, 2, 0.0, 0.1).with_tap(1.05, 0.0);
        assert!((b.tap_ratio - 1.05).abs() < 1e-12);
    }

    #[test]
    fn branch_serde_roundtrip() {
        let b = Branch::new(1, "L1", 1, 2, 0.01, 0.1).with_susceptance(0.02);
        let s = serde_json::to_string(&b).unwrap();
        let d: Branch = serde_json::from_str(&s).unwrap();
        assert_eq!(d.id, 1);
        assert_eq!(d.from_bus, 1);
        assert_eq!(d.to_bus, 2);
        assert!((d.susceptance_pu - 0.02).abs() < 1e-12);
    }
}
