//! # tpt-nrg-powerflow
//!
//! Power-flow solvers for TPT Energy: Newton–Raphson, Gauss–Seidel, Fast
//! Decoupled, and DC power flow.
//!
//! The [`PowerFlowSolver::solve`] entry point dispatches to the requested
//! method and returns a [`PowerFlowResult`] with bus voltages, branch flows,
//! and total system losses.

#![deny(missing_docs)]

mod dc;
mod gauss_seidel;
mod newton_raphson;
mod result;
mod solver;
mod util;

pub use result::{BranchFlow, PowerFlowResult};
pub use solver::{PowerFlowError, PowerFlowMethod, PowerFlowOptions, PowerFlowSolver};
