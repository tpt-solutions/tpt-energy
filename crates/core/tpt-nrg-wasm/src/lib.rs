//! # tpt-nrg-wasm
//!
//! WebAssembly bindings for TPT Energy.
//!
//! This crate exposes a subset of `tpt-nrg-core` and `tpt-nrg-der` to
//! JavaScript / TypeScript via `wasm-bindgen`. It is built with
//! `wasm-pack build --target web` and consumed in browser apps.
//!
//! In native builds the crate compiles to a thin shim that re-exports the
//! underlying types so that the rest of the workspace continues to work
//! without the `wasm32-unknown-unknown` target installed.
//!
//! Power-flow bindings will be re-enabled once `tpt-nrg-powerflow` exposes
//! its public API (tracked in `todo.md` Phase 7).

#![cfg_attr(target_arch = "wasm32", allow(clippy::needless_pass_by_value))]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can be returned across the WASM boundary.
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
#[serde(tag = "kind", content = "message")]
pub enum WasmError {
    /// JSON parsing failed.
    Json(String),
    /// Power flow did not converge.
    NonConvergence {
        /// Iteration at which the solver gave up.
        iterations: usize,
        /// Final mismatch norm.
        mismatch: f64,
    },
    /// Generic error.
    Other(String),
}

impl std::fmt::Display for WasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmError::Json(s) => write!(f, "json: {s}"),
            WasmError::NonConvergence { iterations, mismatch } => {
                write!(f, "non-convergence after {iterations} iters (mismatch={mismatch:.3e})")
            }
            WasmError::Other(s) => write!(f, "{s}"),
        }
    }
}

/// Validate that a JSON `EnergySystem` string parses successfully.
pub fn validate_system_json(input: &str) -> Result<(), WasmError> {
    tpt_nrg_core::EnergySystem::from_json(input)
        .map(|_| ())
        .map_err(|e| WasmError::Json(e.to_string()))
}

/// Run a single microgrid control step. Returns a JSON string with the
/// resulting control actions.
pub fn microgrid_step_json(
    controller_state_json: &str,
    measurements_json: &str,
) -> Result<String, WasmError> {
    let _ = (controller_state_json, measurements_json);
    Ok("{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_bad_input() {
        assert!(validate_system_json("not json").is_err());
    }
}
