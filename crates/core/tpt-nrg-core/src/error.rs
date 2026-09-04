//! Error types for `tpt-nrg-core`.

use thiserror::Error;

/// Result alias for `tpt-nrg-core`.
pub type CoreResult<T> = Result<T, CoreError>;

/// Errors that can occur when constructing or validating an [`EnergySystem`].
#[derive(Debug, Error)]
pub enum CoreError {
    /// A bus with the given id was not found.
    #[error("bus {0} not found")]
    BusNotFound(usize),

    /// A branch references a bus that is not present in the system.
    #[error("branch {branch_id} references unknown bus {bus_id}")]
    UnknownBusForBranch {
        /// Branch identifier.
        branch_id: usize,
        /// Bus identifier that was missing.
        bus_id: usize,
    },

    /// A generator references a bus that is not present in the system.
    #[error("generator {generator_id} references unknown bus {bus_id}")]
    UnknownBusForGenerator {
        /// Generator identifier.
        generator_id: usize,
        /// Bus identifier that was missing.
        bus_id: usize,
    },

    /// A load references a bus that is not present in the system.
    #[error("load {load_id} references unknown bus {bus_id}")]
    UnknownBusForLoad {
        /// Load identifier.
        load_id: usize,
        /// Bus identifier that was missing.
        bus_id: usize,
    },

    /// A storage unit references a bus that is not present in the system.
    #[error("storage {storage_id} references unknown bus {bus_id}")]
    UnknownBusForStorage {
        /// Storage identifier.
        storage_id: usize,
        /// Bus identifier that was missing.
        bus_id: usize,
    },

    /// A duplicate identifier was used.
    #[error("duplicate id {0} for {1}")]
    DuplicateId(usize, &'static str),

    /// The system has no slack bus, which is required for power flow.
    #[error("system has no slack bus; at least one is required for power flow")]
    NoSlackBus,

    /// Multiple slack buses were found.
    #[error("system has {0} slack buses; exactly one is required")]
    MultipleSlackBuses(usize),

    /// JSON (de)serialization failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
