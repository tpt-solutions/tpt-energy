//! # tpt-nrg-topology
//!
//! Network topology utilities for TPT Energy: graph construction, connected
//! components (island detection), shortest path, and Y-bus admittance matrix
//! construction.

#![deny(missing_docs)]

mod admittance;
mod graph;

#[cfg(feature = "substrate")]
pub use admittance::build_sparse_coo;
pub use admittance::{AdmittanceMatrix, AdmittanceMatrixBuilder};
pub use graph::{NetworkTopology, PathResult};

use thiserror::Error;

/// Result alias for `tpt-nrg-topology`.
pub type TopologyResult<T> = Result<T, TopologyError>;

/// Errors for topology operations.
#[derive(Debug, Error)]
pub enum TopologyError {
    /// The bus id was not found.
    #[error("bus {0} not found in topology")]
    BusNotFound(usize),
}
