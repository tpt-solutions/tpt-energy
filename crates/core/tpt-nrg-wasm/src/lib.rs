//! # tpt-nrg-wasm
//!
//! `WebAssembly` bindings for TPT Energy.
//!
//! This crate exposes power flow, validation, economic dispatch, unit
//! commitment, LCOE, carbon intensity, SVG rendering, and a microgrid control
//! loop to `JavaScript` / `TypeScript` via `wasm-bindgen`. It is built with
//! `wasm-pack build --target web` and consumed in browser apps.
//!
//! Every exported function is JSON-in / JSON-out and returns
//! [`WasmError`], whose `kind` field is a stable, machine-readable slug so a
//! JS caller can branch on the failure without parsing message text.
//!
//! In native builds the crate compiles to a thin shim that re-exports the
//! underlying types so that the rest of the workspace continues to work
//! without the `wasm32-unknown-unknown` target installed.

#![cfg_attr(target_arch = "wasm32", allow(clippy::needless_pass_by_value))]

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod control;

pub use control::{ControlConfig, ControlDecision, Measurements};

/// Machine-readable classification of a WASM boundary failure.
///
/// These slugs are part of the public `JavaScript` API: switch on `kind`, never
/// on `message` text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmErrorKind {
    /// The input was not valid JSON.
    Json,
    /// The JSON parsed but is not a valid energy system.
    Validation,
    /// A power-flow solve did not converge.
    NonConvergence,
    /// A dispatch or commitment problem was infeasible.
    Infeasible,
    /// The control loop was given inconsistent measurements.
    Control,
    /// Serializing the result failed.
    Serialization,
}

impl WasmErrorKind {
    /// Stable lower-case slug, mirrored in the `JavaScript` type definitions.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Validation => "validation",
            Self::NonConvergence => "non_convergence",
            Self::Infeasible => "infeasible",
            Self::Control => "control",
            Self::Serialization => "serialization",
        }
    }
}

impl std::fmt::Display for WasmErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors returned across the WASM boundary.
///
/// The JSON shape is `{ "kind": <slug>, "message": <text> }`, so a `JavaScript`
/// caller can `switch (err.kind)` and fall back to `err.message` for display.
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub struct WasmError {
    /// Machine-readable classification.
    pub kind: WasmErrorKind,
    /// Human-readable detail.
    pub message: String,
}

impl WasmError {
    /// Build an error of a given kind.
    #[must_use]
    pub fn new(kind: WasmErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// A JSON parse or validation failure.
    #[must_use]
    pub fn json(message: impl Into<String>) -> Self {
        Self::new(WasmErrorKind::Json, message)
    }

    /// A structurally invalid system.
    #[must_use]
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(WasmErrorKind::Validation, message)
    }
}

impl std::fmt::Display for WasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl From<tpt_nrg_core::CoreError> for WasmError {
    fn from(e: tpt_nrg_core::CoreError) -> Self {
        Self::validation(e.to_string())
    }
}

impl From<tpt_nrg_powerflow::PowerFlowError> for WasmError {
    fn from(e: tpt_nrg_powerflow::PowerFlowError) -> Self {
        use tpt_nrg_powerflow::PowerFlowError as E;
        match e {
            E::NonConvergence {
                iterations,
                mismatch,
            } => Self::new(
                WasmErrorKind::NonConvergence,
                format!("did not converge after {iterations} iterations (mismatch {mismatch:.3e})"),
            ),
            other => Self::validation(other.to_string()),
        }
    }
}

impl From<tpt_nrg_economic_dispatch::DispatchError> for WasmError {
    fn from(e: tpt_nrg_economic_dispatch::DispatchError) -> Self {
        Self::new(WasmErrorKind::Infeasible, e.to_string())
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
    /// Per-branch apparent flow (MVA), in branch order.
    pub branch_flow_mva: Vec<f64>,
    /// Per-branch loading as a fraction of rating, in branch order.
    pub branch_loading: Vec<f64>,
}

/// Per-asset control action emitted by [`microgrid_step_json`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlAction {
    /// Asset identifier.
    pub asset_id: String,
    /// Asset kind, echoed from the state.
    pub kind: String,
    /// Target active power output (MW, positive = generation).
    pub target_p_mw: f64,
    /// Target reactive power output (`MVAr`).
    pub target_q_mvar: f64,
    /// State of charge after the step, for storage assets.
    pub new_state_of_charge: Option<f64>,
    /// What the controller decided: `hold`, `charge`, `discharge`,
    /// `setpoint`, or `shed`.
    pub action: String,
}

/// Microgrid controller state, serialisable across the WASM boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrogridState {
    /// Per-asset state (id, kind, current output in MW, SoC if applicable).
    pub assets: Vec<MicrogridAsset>,
    /// Whether the microgrid is grid-connected.
    pub grid_connected: bool,
}

/// A single asset inside a microgrid state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrogridAsset {
    /// Asset id (e.g. "solar-1").
    pub id: String,
    /// Asset kind: `solar`, `wind`, `battery`, `load`, `diesel`, or `grid`.
    pub kind: String,
    /// Current output (MW, signed: positive = generation, negative = load).
    pub output_mw: f64,
    /// State of charge for storage assets (0-1).
    pub state_of_charge: Option<f64>,
    /// Capacity (MW).
    pub capacity_mw: f64,
    /// Energy capacity (MWh); for a battery this is `capacity_mw` hours of
    /// discharge, so a 1-hour system is the default.
    #[serde(default)]
    pub energy_capacity_mwh: Option<f64>,
}

impl MicrogridAsset {
    /// True for a weather-dependent renewable that should run at its output.
    #[must_use]
    pub fn is_renewable(&self) -> bool {
        matches!(
            self.kind.to_ascii_lowercase().as_str(),
            "solar" | "pv" | "wind"
        )
    }

    /// True for a battery.
    #[must_use]
    pub fn is_battery(&self) -> bool {
        matches!(
            self.kind.to_ascii_lowercase().as_str(),
            "battery" | "storage"
        )
    }

    /// True for a dispatchable generator.
    #[must_use]
    pub fn is_dispatchable(&self) -> bool {
        matches!(
            self.kind.to_ascii_lowercase().as_str(),
            "diesel" | "genset" | "generator" | "thermal"
        )
    }

    /// True for a controllable load.
    #[must_use]
    pub fn is_load(&self) -> bool {
        matches!(self.kind.to_ascii_lowercase().as_str(), "load" | "demand")
    }

    /// Energy capacity in MWh, defaulting to one hour of rated power.
    #[must_use]
    pub fn energy_capacity_mwh(&self) -> f64 {
        self.energy_capacity_mwh
            .unwrap_or(self.capacity_mw)
            .max(f64::EPSILON)
    }
}

/// Parse a JSON `EnergySystem`, rejecting anything structurally invalid.
fn parse_system(input: &str) -> Result<tpt_nrg_core::EnergySystem, WasmError> {
    let system =
        tpt_nrg_core::EnergySystem::from_json(input).map_err(|e| WasmError::json(e.to_string()))?;
    system.validate().map_err(WasmError::from)?;
    Ok(system)
}

/// Serialize a value, mapping a failure onto the `serialization` kind.
fn to_json<T: Serialize>(value: &T) -> Result<String, WasmError> {
    serde_json::to_string(value)
        .map_err(|e| WasmError::new(WasmErrorKind::Serialization, e.to_string()))
}

/// Parse a JSON request object with a `system` field.
fn parse_request<T: serde::de::DeserializeOwned>(
    request: &str,
    what: &str,
) -> Result<T, WasmError> {
    serde_json::from_str(request).map_err(|e| WasmError::json(format!("{what}: {e}")))
}

/// Run a Newton-Raphson power flow from a JSON `EnergySystem` string.
///
/// # Errors
///
/// Returns a [`WasmError`] of kind `json` if `input` is not valid JSON, of
/// kind `validation` if it is not a well-formed system, and of kind
/// `non_convergence` if the solver does not converge.
pub fn run_powerflow_json(input: &str) -> Result<WasmPowerFlowResult, WasmError> {
    let system = parse_system(input)?;
    let solver =
        tpt_nrg_powerflow::PowerFlowSolver::new(tpt_nrg_powerflow::PowerFlowMethod::NewtonRaphson);
    let result = solver.solve(&system)?;
    Ok(WasmPowerFlowResult {
        converged: result.converged,
        iterations: result.iterations,
        voltage_magnitude_pu: result.bus_voltage_magnitude_pu,
        voltage_angle_rad: result.bus_voltage_angle_rad,
        losses_mw: result.total_losses_mw,
        branch_flow_mva: result
            .branch_flows
            .iter()
            .map(|f| (f.p_from_mw * f.p_from_mw + f.q_from_mvar * f.q_from_mvar).sqrt())
            .collect(),
        branch_loading: result
            .branch_flows
            .iter()
            .map(|f| f.loading_fraction)
            .collect(),
    })
}

/// Validate that a JSON `EnergySystem` string parses and passes validation.
///
/// # Errors
///
/// Returns a [`WasmError`] of kind `json` or `validation`.
pub fn validate_system_json(input: &str) -> Result<(), WasmError> {
    parse_system(input).map(|_| ())
}

/// Request body of [`economic_dispatch_json`].
#[derive(Debug, Deserialize)]
struct DispatchRequest {
    /// JSON `EnergySystem`.
    system: String,
    /// System load in MW.
    load_mw: f64,
}

/// Request body of [`unit_commitment_json`].
#[derive(Debug, Deserialize)]
struct CommitmentRequest {
    /// JSON `EnergySystem`.
    system: String,
    /// Load in MW per interval.
    load_profile_mw: Vec<f64>,
}

/// Request body of [`lcoe_json`].
#[derive(Debug, Deserialize)]
struct LcoeRequest {
    /// JSON `EnergySystem`, used for the reported capacity factor.
    system: String,
    /// Overnight capital cost in dollars.
    #[serde(default)]
    capex_dollar: f64,
    /// Annual fixed O&M in dollars per year.
    #[serde(default)]
    annual_fixed_om_dollar: f64,
    /// Variable O&M in dollars per MWh.
    #[serde(default)]
    variable_om_dollar_per_mwh: f64,
    /// Fuel cost in dollars per MWh.
    #[serde(default)]
    fuel_cost_dollar_per_mwh: f64,
    /// Annual energy production in MWh.
    annual_energy_mwh: f64,
    /// Project lifetime in years.
    #[serde(default = "default_lifetime")]
    lifetime_years: u32,
    /// Annual discount rate.
    #[serde(default = "default_discount_rate")]
    discount_rate: f64,
    /// Optional capacity factor override; derived from the system when absent.
    #[serde(default)]
    capacity_factor: Option<f64>,
}

fn default_lifetime() -> u32 {
    25
}

fn default_discount_rate() -> f64 {
    0.07
}

/// Request body of [`visualize_json`].
#[derive(Debug, Deserialize)]
struct VizRequest {
    /// JSON `EnergySystem`.
    system: String,
    /// Draw the voltage/loading legend.
    #[serde(default = "default_true")]
    show_legend: bool,
    /// Draw per-branch MW flow labels.
    #[serde(default = "default_true")]
    show_flow_labels: bool,
}

fn default_true() -> bool {
    true
}

/// Run a lossless economic dispatch, returning the result as JSON.
///
/// `request` is `{"system": <json>, "load_mw": <number>}`.
///
/// # Errors
///
/// Returns a [`WasmError`] of kind `json`, `validation`, or `infeasible`.
pub fn economic_dispatch_json(request: &str) -> Result<String, WasmError> {
    let request: DispatchRequest = parse_request(request, "dispatch request")?;
    let system = parse_system(&request.system)?;
    let result = tpt_nrg_economic_dispatch::economic_dispatch(&system, request.load_mw)?;
    to_json(&serde_json::json!({
        "generator_outputs_mw": result.generator_outputs_mw,
        "total_cost_dollar_per_h": result.total_cost_dollar_per_h,
        "marginal_cost_dollar_per_mwh": result.marginal_cost_dollar_per_mwh,
        "losses_mw": result.losses_mw,
    }))
}

/// Run a priority-list unit commitment over a load profile, returning JSON.
///
/// `request` is `{"system": <json>, "load_profile_mw": [<number>, ...]}`.
///
/// # Errors
///
/// Returns a [`WasmError`] of kind `json` or `validation`.
pub fn unit_commitment_json(request: &str) -> Result<String, WasmError> {
    let request: CommitmentRequest = parse_request(request, "commitment request")?;
    let system = parse_system(&request.system)?;
    if request.load_profile_mw.is_empty() {
        return Err(WasmError::validation(
            "load_profile_mw must contain at least one interval",
        ));
    }
    let result = tpt_nrg_unit_commitment::unit_commitment(&system, &request.load_profile_mw);
    to_json(&serde_json::json!({
        "commitment": result.commitment,
        "outputs_mw": result.outputs,
        "total_cost_dollar": result.total_cost_dollar,
    }))
}

/// Compute the levelized cost of energy, returning JSON.
///
/// `request` is `{"system": <json>, "capex_dollar": <number>,
/// "annual_fixed_om_dollar": <number>, "variable_om_dollar_per_mwh": <number>,
/// "fuel_cost_dollar_per_mwh": <number>, "annual_energy_mwh": <number>,
/// "lifetime_years": <integer>, "discount_rate": <number>}`; every field
/// except `system` and `annual_energy_mwh` is optional.
///
/// # Errors
///
/// Returns a [`WasmError`] of kind `json`, or `validation` when the annual
/// energy is not a positive finite number.
pub fn lcoe_json(request: &str) -> Result<String, WasmError> {
    let request: LcoeRequest = parse_request(request, "lcoe request")?;
    if !request.annual_energy_mwh.is_finite() || request.annual_energy_mwh <= 0.0 {
        return Err(WasmError::validation(
            "annual_energy_mwh must be a positive finite number",
        ));
    }
    let system = parse_system(&request.system)?;
    let inputs = tpt_nrg_lcoe::LcoeInputs {
        capex_dollar: request.capex_dollar,
        annual_fixed_om_dollar: request.annual_fixed_om_dollar,
        variable_om_dollar_per_mwh: request.variable_om_dollar_per_mwh,
        fuel_cost_dollar_per_mwh: request.fuel_cost_dollar_per_mwh,
        annual_energy_mwh: request.annual_energy_mwh,
        lifetime_years: request.lifetime_years,
        discount_rate: request.discount_rate,
        capacity_factor: request.capacity_factor.unwrap_or_else(|| {
            let rated = system.base_mva * 8760.0;
            if rated > 0.0 {
                request.annual_energy_mwh / rated
            } else {
                0.0
            }
        }),
    };
    to_json(&serde_json::json!({
        "lcoe_dollar_per_mwh": tpt_nrg_lcoe::levelized_cost_of_energy(&inputs),
    }))
}

/// Report the carbon intensity of a system's scheduled dispatch, as JSON.
///
/// # Errors
///
/// Returns a [`WasmError`] of kind `json` or `validation`.
pub fn carbon_intensity_json(input: &str) -> Result<String, WasmError> {
    let system = parse_system(input)?;
    to_json(&serde_json::json!({
        "carbon_intensity_kg_per_mwh": tpt_nrg_carbon::carbon_intensity(&system),
    }))
}

/// Render a single-line diagram and heatmap for a system, as SVG.
///
/// `request` is `{"system": <json>, "show_legend": <bool>,
/// "show_flow_labels": <bool>}`; both flags default to `true`.
///
/// # Errors
///
/// Returns a [`WasmError`] of kind `json` or `validation`. A power flow that
/// fails to converge is *not* an error: the diagram falls back to the
/// scheduled voltages, because a picture of the case is still useful.
pub fn visualize_json(request: &str) -> Result<String, WasmError> {
    let request: VizRequest = parse_request(request, "visualize request")?;
    let system = parse_system(&request.system)?;
    let result =
        tpt_nrg_powerflow::PowerFlowSolver::new(tpt_nrg_powerflow::PowerFlowMethod::NewtonRaphson)
            .solve(&system)
            .ok();
    let options = tpt_nrg_viz::VizOptions {
        title: String::new(),
        show_legend: request.show_legend,
        show_flow_labels: request.show_flow_labels,
        ..tpt_nrg_viz::VizOptions::default()
    };
    Ok(tpt_nrg_viz::render(&system, result.as_ref(), &options))
}

/// Run one microgrid control step, returning the decision as JSON.
///
/// `controller_state_json` is a [`MicrogridState`]; `measurements_json` is a
/// [`Measurements`]. The optional `config_json` tunes the droops;
/// omitted fields fall back to [`ControlConfig::default`].
///
/// # Errors
///
/// Returns a [`WasmError`] of kind `json` if any input is not valid JSON, of
/// kind `control` if the measurements are unusable, and of kind
/// `serialization` if the decision cannot be encoded.
pub fn microgrid_step_json(
    controller_state_json: &str,
    measurements_json: &str,
    config_json: Option<&str>,
) -> Result<String, WasmError> {
    let state: MicrogridState = parse_request(controller_state_json, "microgrid state")?;
    let measurements: control::Measurements = parse_request(measurements_json, "measurements")?;
    let config: ControlConfig = match config_json {
        None => ControlConfig::default(),
        Some(text) => parse_request(text, "control config")?,
    };
    to_json(&control::microgrid_step(&state, &measurements, &config)?)
}

// ---------------------------------------------------------------------------
// `wasm-bindgen` exports - only compiled when the `wasm` feature is enabled.
// On native builds these are absent and JS interop goes through the JSON
// functions above, which is what the test suite exercises.
// ---------------------------------------------------------------------------
#[cfg(feature = "wasm")]
mod wasm_bindings {
    use serde::Serialize;
    use wasm_bindgen::prelude::*;

    use super::{
        carbon_intensity_json, economic_dispatch_json, lcoe_json, microgrid_step_json,
        run_powerflow_json, unit_commitment_json, validate_system_json, visualize_json, WasmError,
    };

    /// Serialise a result to a JS value, or throw a typed `WasmError` object.
    fn ok<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(value).map_err(|e| {
            serde_wasm_bindgen::to_value(&WasmError::new(
                super::WasmErrorKind::Serialization,
                e.to_string(),
            ))
            .unwrap_or_else(|_| JsValue::from_str("serialization error"))
        })
    }

    /// Convert a `WasmError` into the JS object callers branch on.
    fn err(e: &WasmError) -> JsValue {
        serde_wasm_bindgen::to_value(&e)
            .unwrap_or_else(|_| JsValue::from_str(&format!("{}: {}", e.kind, e.message)))
    }

    #[wasm_bindgen(start)]
    pub fn _start() {
        // Module initialisation hook. Currently a no-op.
    }

    /// Run a Newton-Raphson power flow from a JSON `EnergySystem`.
    ///
    /// Throws an object `{ kind, message }` on failure.
    #[wasm_bindgen]
    pub fn wasm_run_powerflow_json(input: &str) -> Result<JsValue, JsValue> {
        run_powerflow_json(input)
            .map_err(|e| err(&e))
            .and_then(|r| ok(&r))
    }

    /// Validate a JSON `EnergySystem`, throwing a typed error if it is invalid.
    #[wasm_bindgen]
    pub fn wasm_validate_system_json(input: &str) -> Result<(), JsValue> {
        validate_system_json(input).map_err(|e| err(&e))
    }

    /// Run a lossless economic dispatch. See the Rust docs for the request shape.
    #[wasm_bindgen]
    pub fn wasm_economic_dispatch_json(request: &str) -> Result<JsValue, JsValue> {
        economic_dispatch_json(request)
            .map_err(|e| err(&e))
            .and_then(|s| ok(&s))
    }

    /// Run a priority-list unit commitment. See the Rust docs for the request shape.
    #[wasm_bindgen]
    pub fn wasm_unit_commitment_json(request: &str) -> Result<JsValue, JsValue> {
        unit_commitment_json(request)
            .map_err(|e| err(&e))
            .and_then(|s| ok(&s))
    }

    /// Compute the levelized cost of energy. See the Rust docs for the request shape.
    #[wasm_bindgen]
    pub fn wasm_lcoe_json(request: &str) -> Result<JsValue, JsValue> {
        lcoe_json(request).map_err(|e| err(&e)).and_then(|s| ok(&s))
    }

    /// Report the carbon intensity of a system, as kg CO2 per MWh.
    #[wasm_bindgen]
    pub fn wasm_carbon_intensity_json(input: &str) -> Result<JsValue, JsValue> {
        carbon_intensity_json(input)
            .map_err(|e| err(&e))
            .and_then(|s| ok(&s))
    }

    /// Render a single-line diagram and heatmap as an SVG string.
    #[wasm_bindgen]
    pub fn wasm_visualize_json(request: &str) -> Result<String, JsValue> {
        visualize_json(request).map_err(|e| err(&e))
    }

    /// Run one microgrid control step and return the decision as JSON.
    #[wasm_bindgen]
    #[allow(clippy::needless_pass_by_value)]
    pub fn wasm_microgrid_step_json(
        controller_state_json: &str,
        measurements_json: &str,
        config_json: Option<String>,
    ) -> Result<String, JsValue> {
        microgrid_step_json(
            controller_state_json,
            measurements_json,
            config_json.as_deref(),
        )
        .map_err(|e| err(&e))
    }

    /// `WasmMicrogridController` - a JS-friendly wrapper that holds a
    /// `MicrogridState` and emits control actions on each step.
    #[wasm_bindgen]
    pub struct WasmMicrogridController {
        state_json: String,
    }

    #[wasm_bindgen]
    impl WasmMicrogridController {
        /// Construct a controller from a JSON state, validating it up front.
        #[wasm_bindgen(constructor)]
        pub fn new(state_json: &str) -> Result<WasmMicrogridController, JsValue> {
            // Validating up front turns a bad state into a constructor error
            // rather than a failure on every step.
            serde_json::from_str::<super::MicrogridState>(state_json)
                .map_err(|e| err(&WasmError::json(format!("microgrid state: {e}"))))?;
            Ok(Self {
                state_json: state_json.to_string(),
            })
        }

        /// Run one control step and return the actions as JSON.
        #[wasm_bindgen]
        #[allow(clippy::needless_pass_by_value)]
        pub fn step(
            &self,
            measurements_json: &str,
            config_json: Option<String>,
        ) -> Result<String, JsValue> {
            microgrid_step_json(&self.state_json, measurements_json, config_json.as_deref())
                .map_err(|e| err(&e))
        }

        /// The current state JSON.
        #[wasm_bindgen(getter)]
        pub fn state_json(&self) -> String {
            self.state_json.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A two-bus system with a cost curve, enough for every entry point.
    const CASE: &str = r#"{
        "id": "two", "name": "Two bus", "base_mva": 100.0, "frequency_hz": 60.0,
        "buses": [
            {"id": 1, "name": "Slack", "type": "Slack", "voltage_magnitude_pu": 1.06},
            {"id": 2, "name": "Load", "type": "Pq", "load_mw": 30.0, "load_mvar": 10.0}
        ],
        "branches": [
            {"id": 1, "name": "L12", "from_bus": 1, "to_bus": 2,
             "resistance_pu": 0.01, "reactance_pu": 0.05, "rating_mva": 100.0}
        ],
        "generators": [
            {"id": 1, "name": "G1", "bus_id": 1, "type": "Thermal",
             "p_max_mw": 100.0, "p_min_mw": 10.0, "p_schedule_mw": 40.0,
             "cost_curve": {"no_load_cost": 0.0, "startup_cost": 0.0,
               "segments": [{"start_mw": 10.0, "end_mw": 100.0,
                             "incremental_cost_per_mwh": 25.0}]}}
        ]
    }"#;

    const STATE: &str = r#"{
        "assets": [
            {"id": "s1", "kind": "solar", "output_mw": 5.0,
             "state_of_charge": null, "capacity_mw": 10.0},
            {"id": "b1", "kind": "battery", "output_mw": 0.0,
             "state_of_charge": 0.8, "capacity_mw": 5.0},
            {"id": "l1", "kind": "load", "output_mw": 0.0,
             "state_of_charge": null, "capacity_mw": 10.0}
        ],
        "grid_connected": true
    }"#;

    #[test]
    fn error_kinds_have_stable_slugs() {
        assert_eq!(WasmErrorKind::Json.as_str(), "json");
        assert_eq!(WasmErrorKind::Validation.as_str(), "validation");
        assert_eq!(WasmErrorKind::NonConvergence.as_str(), "non_convergence");
        assert_eq!(WasmErrorKind::Infeasible.as_str(), "infeasible");
        assert_eq!(WasmErrorKind::Control.as_str(), "control");
        assert_eq!(WasmErrorKind::Serialization.as_str(), "serialization");
        assert_eq!(WasmErrorKind::Json.to_string(), "json");
    }

    #[test]
    fn errors_serialize_as_kind_and_message() {
        let e = WasmError::validation("bad system");
        let json = serde_json::to_value(&e).expect("serialize");
        assert_eq!(json["kind"], "validation");
        assert_eq!(json["message"], "bad system");
        assert!(e.to_string().contains("validation"));
    }

    #[test]
    fn validate_rejects_bad_input() {
        let err = validate_system_json("not json").unwrap_err();
        assert_eq!(err.kind, WasmErrorKind::Json);
    }

    #[test]
    fn validate_accepts_a_good_system() {
        validate_system_json(CASE).expect("case is valid");
    }

    #[test]
    fn validate_rejects_a_system_without_a_slack_bus() {
        let bad = r#"{"id":"x","name":"x","base_mva":100.0,"frequency_hz":60.0,
            "buses":[{"id":1,"name":"B","type":"Pq"}],
            "branches":[],"generators":[]}"#;
        let err = validate_system_json(bad).unwrap_err();
        assert_eq!(err.kind, WasmErrorKind::Validation);
    }

    #[test]
    fn run_powerflow_json_rejects_bad_input() {
        let err = run_powerflow_json("not json").unwrap_err();
        assert_eq!(err.kind, WasmErrorKind::Json);
    }

    #[test]
    fn run_powerflow_json_solves_a_case() {
        let r = run_powerflow_json(CASE).expect("solves");
        assert!(r.converged);
        assert_eq!(r.voltage_magnitude_pu.len(), 2);
        assert_eq!(r.branch_flow_mva.len(), 1);
        assert_eq!(r.branch_loading.len(), 1);
        assert!(r.losses_mw > 0.0, "a lossy line dissipates something");
    }

    #[test]
    fn economic_dispatch_json_solves_a_case() {
        let request = serde_json::json!({ "system": CASE, "load_mw": 40.0 }).to_string();
        let out = economic_dispatch_json(&request).expect("dispatch");
        let v: serde_json::Value = serde_json::from_str(&out).expect("json");
        assert!(v["generator_outputs_mw"][0].as_f64().unwrap() > 30.0);
        assert!(v["marginal_cost_dollar_per_mwh"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn economic_dispatch_json_reports_infeasibility() {
        let request = serde_json::json!({ "system": CASE, "load_mw": 10_000.0 }).to_string();
        let err = economic_dispatch_json(&request).unwrap_err();
        assert_eq!(err.kind, WasmErrorKind::Infeasible);
    }

    #[test]
    fn unit_commitment_json_solves_a_case() {
        let request = serde_json::json!({ "system": CASE, "load_profile_mw": [30.0, 60.0, 20.0] })
            .to_string();
        let out = unit_commitment_json(&request).expect("commitment");
        let v: serde_json::Value = serde_json::from_str(&out).expect("json");
        assert_eq!(v["commitment"][0].as_array().unwrap().len(), 3);
    }

    #[test]
    fn unit_commitment_json_rejects_an_empty_profile() {
        let request = serde_json::json!({ "system": CASE, "load_profile_mw": [] }).to_string();
        let err = unit_commitment_json(&request).unwrap_err();
        assert_eq!(err.kind, WasmErrorKind::Validation);
    }

    #[test]
    fn lcoe_json_computes_a_cost() {
        let request = serde_json::json!({
            "system": CASE,
            "capex_dollar": 1.0e8,
            "annual_energy_mwh": 50_000.0,
            "lifetime_years": 20,
            "discount_rate": 0.07
        })
        .to_string();
        let out = lcoe_json(&request).expect("lcoe");
        let v: serde_json::Value = serde_json::from_str(&out).expect("json");
        assert!(v["lcoe_dollar_per_mwh"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn lcoe_json_rejects_a_non_positive_energy() {
        let request = serde_json::json!({ "system": CASE, "annual_energy_mwh": 0.0 }).to_string();
        let err = lcoe_json(&request).unwrap_err();
        assert_eq!(err.kind, WasmErrorKind::Validation);
    }

    #[test]
    fn carbon_intensity_json_reports_kg_per_mwh() {
        let out = carbon_intensity_json(CASE).expect("carbon");
        let v: serde_json::Value = serde_json::from_str(&out).expect("json");
        // A single thermal unit is 900 kg CO2/MWh by the crate's factor.
        assert!((v["carbon_intensity_kg_per_mwh"].as_f64().unwrap() - 900.0).abs() < 1.0);
    }

    #[test]
    fn visualize_json_renders_svg() {
        let request = serde_json::json!({ "system": CASE }).to_string();
        let svg = visualize_json(&request).expect("svg");
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn microgrid_step_json_produces_actions() {
        let out = microgrid_step_json(STATE, r#"{"demand_mw": 8.0}"#, None).expect("step");
        let v: serde_json::Value = serde_json::from_str(&out).expect("json");
        let actions = v["actions"].as_array().expect("actions");
        assert_eq!(actions.len(), 3);
        assert_eq!(v["balanced"], true);
        let solar = actions
            .iter()
            .find(|a| a["asset_id"] == "s1")
            .expect("solar");
        assert_eq!(solar["action"], "hold");
    }

    #[test]
    fn microgrid_step_json_honours_a_config_override() {
        // Islanded, so the controller has to balance the step itself.
        let island = r#"{
            "assets": [
                {"id": "b1", "kind": "battery", "output_mw": 0.0,
                 "state_of_charge": 0.8, "capacity_mw": 5.0},
                {"id": "l1", "kind": "load", "output_mw": 0.0,
                 "state_of_charge": null, "capacity_mw": 10.0}
            ],
            "grid_connected": false
        }"#;
        // Raising the floor to the current SoC leaves the battery no headroom.
        let config = r#"{"min_state_of_charge": 0.8, "interval_hours": 1.0}"#;
        let out = microgrid_step_json(island, r#"{"demand_mw": 9.0}"#, Some(config)).expect("step");
        let v: serde_json::Value = serde_json::from_str(&out).expect("json");
        assert!(
            v["load_shed_fraction"].as_f64().unwrap() > 0.0,
            "an island with no battery headroom must shed load"
        );
    }

    #[test]
    fn a_grid_connected_step_never_sheds() {
        let out = microgrid_step_json(STATE, r#"{"demand_mw": 500.0}"#, None).expect("step");
        let v: serde_json::Value = serde_json::from_str(&out).expect("json");
        assert_eq!(v["load_shed_fraction"], 0.0);
        assert_eq!(v["balanced"], true, "the utility covers any deficit");
    }

    #[test]
    fn microgrid_step_json_rejects_bad_state() {
        let err = microgrid_step_json("{}", "{}", None).unwrap_err();
        assert_eq!(err.kind, WasmErrorKind::Json);
    }

    #[test]
    fn microgrid_step_json_rejects_bad_measurements() {
        let err = microgrid_step_json(STATE, r#"{"frequency_hz": -1}"#, None).unwrap_err();
        assert_eq!(err.kind, WasmErrorKind::Control);
    }

    #[test]
    fn asset_classification_is_case_insensitive() {
        let make = |kind: &str| MicrogridAsset {
            id: "a".to_string(),
            kind: kind.to_string(),
            output_mw: 0.0,
            state_of_charge: None,
            capacity_mw: 1.0,
            energy_capacity_mwh: None,
        };
        assert!(make("SOLAR").is_renewable());
        assert!(make("Battery").is_battery());
        assert!(make("diesel").is_dispatchable());
        assert!(make("Load").is_load());
        assert!(!make("load").is_renewable());
    }

    #[test]
    fn energy_capacity_defaults_to_one_hour() {
        let a = MicrogridAsset {
            id: "b".to_string(),
            kind: "battery".to_string(),
            output_mw: 0.0,
            state_of_charge: Some(0.5),
            capacity_mw: 4.0,
            energy_capacity_mwh: None,
        };
        assert!((a.energy_capacity_mwh() - 4.0).abs() < 1e-12);
    }
}
