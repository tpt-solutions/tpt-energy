//! # tpt-nrg-wasm
//!
//! WebAssembly bindings for TPT Energy.
//!
//! This crate exposes a subset of `tpt-nrg-core`, `tpt-nrg-powerflow`, and
//! `tpt-nrg-der` to JavaScript / TypeScript via `wasm-bindgen`. It is built
//! with `wasm-pack build --target web` and consumed in browser apps.
//!
//! In native builds the crate compiles to a thin shim that re-exports the
//! underlying types so that the rest of the workspace continues to work
//! without the `wasm32-unknown-unknown` target installed.

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

/// A JSON-friendly power-flow result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPowerFlowResult {
    /// Whether the solver converged.
    pub converged: bool,
    /// Iteration count.
    pub iterations: usize,
    /// Per-bus voltage magnitude (pu).
    pub voltage_magnitude_pu: Vec<f64>,
    /// Per-bus voltage angle (radians).
    pub voltage_angle_rad: Vec<f64>,
    /// Total system losses in MW.
    pub losses_mw: f64,
}

/// Per-asset control action emitted by `microgrid_step_json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlAction {
    /// Asset identifier.
    pub asset_id: String,
    /// Target active power output (MW, positive = generation).
    pub target_p_mw: f64,
    /// Target reactive power output (MVAr).
    pub target_q_mvar: f64,
    /// Target voltage setpoint (pu) if applicable.
    pub voltage_setpoint_pu: Option<f64>,
}

/// Microgrid controller state, serialisable across the WASM boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrogridState {
    /// Per-asset state (id, type, current output in MW, SoC if applicable).
    pub assets: Vec<MicrogridAsset>,
    /// Whether the microgrid is grid-connected.
    pub grid_connected: bool,
}

/// A single asset inside a microgrid state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrogridAsset {
    /// Asset id (e.g. "solar-1").
    pub id: String,
    /// Asset type: "solar", "wind", "battery", "load", "diesel".
    pub kind: String,
    /// Current output (MW, signed: positive = generation, negative = load).
    pub output_mw: f64,
    /// State of charge for storage assets (0-1).
    pub state_of_charge: Option<f64>,
    /// Capacity (MW).
    pub capacity_mw: f64,
}

/// Run a Newton-Raphson power flow from a JSON `EnergySystem` string.
pub fn run_powerflow_json(input: &str) -> Result<WasmPowerFlowResult, WasmError> {
    let system = tpt_nrg_core::EnergySystem::from_json(input)
        .map_err(|e| WasmError::Json(e.to_string()))?;
    let solver =
        tpt_nrg_powerflow::PowerFlowSolver::new(tpt_nrg_powerflow::PowerFlowMethod::NewtonRaphson);
    let result = solver
        .solve(&system)
        .map_err(|e| WasmError::Other(e.to_string()))?;
    Ok(WasmPowerFlowResult {
        converged: result.converged,
        iterations: result.iterations,
        voltage_magnitude_pu: result.bus_voltage_magnitude_pu,
        voltage_angle_rad: result.bus_voltage_angle_rad,
        losses_mw: result.total_losses_mw,
    })
}

/// Validate that a JSON `EnergySystem` string parses successfully.
pub fn validate_system_json(input: &str) -> Result<(), WasmError> {
    tpt_nrg_core::EnergySystem::from_json(input)
        .map(|_| ())
        .map_err(|e| WasmError::Json(e.to_string()))
}

/// Run a single microgrid control step. Returns a JSON string with the
/// resulting control actions.
///
/// The current implementation is a placeholder: it parses the controller
/// state, returns the same assets unchanged, and emits no actions. The
/// full control logic (P/f and Q/V droops, grid-forming V/f reference,
/// etc.) is out of local scope and will be added in a later release.
pub fn microgrid_step_json(
    controller_state_json: &str,
    measurements_json: &str,
) -> Result<String, WasmError> {
    let _state: MicrogridState = serde_json::from_str(controller_state_json)
        .map_err(|e| WasmError::Json(e.to_string()))?;
    let _measurements: serde_json::Value = serde_json::from_str(measurements_json)
        .map_err(|e| WasmError::Json(e.to_string()))?;
    let actions: Vec<ControlAction> = Vec::new();
    serde_json::to_string(&actions).map_err(|e| WasmError::Other(e.to_string()))
}

// ---------------------------------------------------------------------------
// `wasm-bindgen` exports - only compiled when the `wasm` feature is enabled
// and the target is `wasm32-unknown-unknown`. On native builds the bindings
// are absent and JS interop is via the plain JSON functions above.
// ---------------------------------------------------------------------------
#[cfg(feature = "wasm")]
mod wasm_bindings {
    use super::*;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn _start() {
        // Initialisation hook for the WASM module. Currently a no-op.
    }

    #[wasm_bindgen]
    pub fn wasm_run_powerflow_json(input: &str) -> Result<JsValue, JsValue> {
        let r = run_powerflow_json(input).map_err(|e| JsValue::from(e.to_string()))?;
        serde_wasm_bindgen::to_value(&r).map_err(|e| JsValue::from(e.to_string()))
    }

    #[wasm_bindgen]
    pub fn wasm_validate_system_json(input: &str) -> Result<(), JsValue> {
        validate_system_json(input).map_err(|e| JsValue::from(e.to_string()))
    }

    #[wasm_bindgen]
    pub fn wasm_microgrid_step_json(
        controller_state_json: &str,
        measurements_json: &str,
    ) -> Result<String, JsValue> {
        microgrid_step_json(controller_state_json, measurements_json)
            .map_err(|e| JsValue::from(e.to_string()))
    }

    /// `WasmMicrogridController` - JS-friendly wrapper that holds a
    /// `MicrogridState` and emits `ControlAction` JSON on each step.
    #[wasm_bindgen]
    pub struct WasmMicrogridController {
        state_json: String,
    }

    #[wasm_bindgen]
    impl WasmMicrogridController {
        /// Construct a new controller from a JSON state string.
        #[wasm_bindgen(constructor)]
        pub fn new(state_json: &str) -> Result<WasmMicrogridController, JsValue> {
            // Validate that the input parses as a MicrogridState.
            let _: MicrogridState =
                serde_json::from_str(state_json).map_err(|e| JsValue::from(e.to_string()))?;
            Ok(Self {
                state_json: state_json.to_string(),
            })
        }

        /// Run one control step and return the actions as a JSON string.
        #[wasm_bindgen]
        pub fn step(&self, measurements_json: &str) -> Result<String, JsValue> {
            microgrid_step_json(&self.state_json, measurements_json)
                .map_err(|e| JsValue::from(e.to_string()))
        }

        /// Get the current state JSON.
        #[wasm_bindgen(getter)]
        pub fn state_json(&self) -> String {
            self.state_json.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_bad_input() {
        assert!(validate_system_json("not json").is_err());
    }

    #[test]
    fn run_powerflow_json_rejects_bad_input() {
        assert!(run_powerflow_json("not json").is_err());
    }

    #[test]
    fn microgrid_step_json_round_trip() {
        let state = r#"{
            "assets": [
                {"id":"s1","kind":"solar","output_mw":5.0,"state_of_charge":null,"capacity_mw":10.0}
            ],
            "grid_connected": true
        }"#;
        let measurements = r#"{}"#;
        let actions_json = microgrid_step_json(state, measurements).expect("microgrid step");
        let parsed: Vec<ControlAction> =
            serde_json::from_str(&actions_json).expect("parse actions");
        assert!(parsed.is_empty(), "placeholder should return no actions");
    }
}
