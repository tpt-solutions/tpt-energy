//! Power-flow result types.

use serde::{Deserialize, Serialize};

/// Full power-flow solution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerFlowResult {
    /// Whether the solver converged.
    pub converged: bool,
    /// Iteration count.
    pub iterations: usize,
    /// Final mismatch norm (per-unit).
    pub final_mismatch: f64,
    /// Per-bus voltage magnitude in per-unit.
    pub bus_voltage_magnitude_pu: Vec<f64>,
    /// Per-bus voltage angle in radians.
    pub bus_voltage_angle_rad: Vec<f64>,
    /// Per-branch flow (in the same order as `EnergySystem::branches`).
    pub branch_flows: Vec<BranchFlow>,
    /// Total system active-power losses in MW.
    pub total_losses_mw: f64,
    /// Total system reactive-power losses in MVAr.
    pub total_losses_mvar: f64,
    /// Per-bus generation dispatch (MW). `Vec` length matches
    /// `EnergySystem::generators`.
    pub generator_p_mw: Vec<f64>,
    /// Per-bus generation dispatch (MVAr).
    pub generator_q_mvar: Vec<f64>,
}

/// Power flow on a single branch, evaluated at the "from" end.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BranchFlow {
    /// Branch id.
    pub id: usize,
    /// Active power flow from "from_bus" to "to_bus" in MW.
    pub p_from_mw: f64,
    /// Reactive power flow from "from_bus" to "to_bus" in MVAr.
    pub q_from_mvar: f64,
    /// Active power flow from "to_bus" to "from_bus" in MW.
    pub p_to_mw: f64,
    /// Reactive power flow from "to_bus" to "from_bus" in MVAr.
    pub q_to_mvar: f64,
    /// Apparent power loading as a fraction of `rating_mva` (0 if unset).
    pub loading_fraction: f64,
}

impl BranchFlow {
    /// True if this branch exceeds 100% of its rating.
    #[must_use]
    pub fn is_overloaded(&self) -> bool {
        self.loading_fraction > 1.0
    }
}
