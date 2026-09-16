//! Common types for power flow.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use tpt_nrg_core::EnergySystem;

use crate::result::PowerFlowResult;

/// Power-flow solution method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerFlowMethod {
    /// Full AC Newton–Raphson iterative solver.
    NewtonRaphson,
    /// AC Gauss–Seidel iterative solver (slower, simpler).
    GaussSeidel,
    /// Fast decoupled power flow (BX / XB variants).
    FastDecoupled,
    /// Linear DC power flow (P = B'·θ).
    DcPowerFlow,
}

/// Power-flow solver options.
#[derive(Debug, Clone, Copy)]
pub struct PowerFlowOptions {
    /// Convergence tolerance for the maximum mismatch (per-unit).
    pub tolerance: f64,
    /// Maximum number of iterations before giving up.
    pub max_iterations: usize,
    /// Acceleration factor for Gauss–Seidel.
    pub acceleration: f64,
}

impl Default for PowerFlowOptions {
    fn default() -> Self {
        Self {
            tolerance: 1e-6,
            max_iterations: 50,
            acceleration: 1.2,
        }
    }
}

/// Errors raised by power-flow solvers.
#[derive(Debug, Error)]
pub enum PowerFlowError {
    /// The system has more than one slack bus.
    #[error("system has {0} slack buses; expected exactly one")]
    MultipleSlackBuses(usize),
    /// The system has no slack bus.
    #[error("system has no slack bus")]
    NoSlackBus,
    /// The solver did not converge within `max_iterations`.
    #[error(
        "power flow did not converge after {iterations} iterations (mismatch = {mismatch:.3e})"
    )]
    NonConvergence {
        /// Iteration count.
        iterations: usize,
        /// Final mismatch norm.
        mismatch: f64,
    },
    /// A bus index was invalid.
    #[error("invalid bus index {0}")]
    InvalidBusIndex(usize),
    /// A reference to a non-existent bus id.
    #[error("unknown bus id {0}")]
    UnknownBusId(usize),
    /// A generic error from the underlying system.
    #[error("{0}")]
    Other(String),
}

/// A configured power-flow solver.
pub struct PowerFlowSolver {
    method: PowerFlowMethod,
    options: PowerFlowOptions,
}

impl PowerFlowSolver {
    /// Create a new solver using the given method and default options.
    #[must_use]
    pub fn new(method: PowerFlowMethod) -> Self {
        Self {
            method,
            options: PowerFlowOptions::default(),
        }
    }

    /// Set the convergence options.
    #[must_use]
    pub fn with_options(mut self, options: PowerFlowOptions) -> Self {
        self.options = options;
        self
    }

    /// Set the convergence tolerance.
    #[must_use]
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.options.tolerance = tolerance;
        self
    }

    /// Set the maximum iteration count.
    #[must_use]
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.options.max_iterations = max_iterations;
        self
    }

    /// Solve the power flow for the given system.
    pub fn solve(&self, system: &EnergySystem) -> Result<PowerFlowResult, PowerFlowError> {
        match self.method {
            PowerFlowMethod::NewtonRaphson => crate::newton_raphson::solve(system, self.options),
            PowerFlowMethod::GaussSeidel => crate::gauss_seidel::solve(system, self.options),
            PowerFlowMethod::FastDecoupled => crate::fast_decoupled::solve(system, self.options),
            PowerFlowMethod::DcPowerFlow => crate::dc::solve(system),
        }
    }
}
