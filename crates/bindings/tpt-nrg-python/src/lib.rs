//! # tpt-nrg-python
//!
//! Python bindings for TPT Energy, built with [`pyo3`] and published with
//! [`maturin`](https://www.maturin.rs/).
//!
//! The module `tpt_nrg` mirrors the Rust API in an idiomatic Python shape:
//!
//! ```python
//! import tpt_nrg
//!
//! system = tpt_nrg.System.from_json(open("ieee14.json").read())
//! result = system.power_flow(tpt_nrg.PowerFlowMethod.NEWTON_RAPHSON)
//!
//! print(result.converged, result.losses_mw)
//! print(system.to_matpower()[:40])
//! ```
//!
//! Errors surface as a single `tpt_nrg.EnergyError` carrying a `kind` slug
//! (`"validation"`, `"non_convergence"`, `"infeasible"`, ...), so Python code
//! can branch on the failure the same way the `JavaScript` bindings do.
//!
//! [pyo3]: https://pyo3.rs

#![deny(missing_docs)]

use pyo3::create_exception;
use pyo3::exceptions::{PyIndexError, PyValueError};
use pyo3::prelude::*;

use tpt_nrg_core::EnergySystem;
use tpt_nrg_economic_dispatch::economic_dispatch;
use tpt_nrg_fault::{FaultAnalyzer, FaultType};
use tpt_nrg_lcoe::{levelized_cost_of_energy, LcoeInputs};
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};
use tpt_nrg_unit_commitment::unit_commitment;

create_exception!(
    tpt_nrg,
    EnergyError,
    PyValueError,
    "An error raised by a TPT Energy operation.\n\n\
     The `kind` attribute is a stable slug: `validation`, `non_convergence`, \
     `infeasible`, `unsupported`, or `parse`."
);

/// Convert a TPT Energy error into a Python exception carrying a `kind`.
fn fail(kind: &str, message: impl std::fmt::Display) -> PyErr {
    let text = message.to_string();
    let err = EnergyError::new_err(format!("{kind}: {text}"));
    // Attach `kind` and `message` so Python can branch without parsing text.
    Python::attach(|py| {
        let value = err.value(py);
        let _ = value.setattr("kind", kind);
        let _ = value.setattr("message", &text);
    });
    err
}

/// Parse a system from text in a named exchange format.
fn parse(text: &str, format: &str) -> PyResult<EnergySystem> {
    let format: tpt_nrg_interop::Format =
        tpt_nrg_interop::Format::parse(format).map_err(|e| fail("unsupported", e))?;
    tpt_nrg_interop::from_text(text, format).map_err(|e| {
        // `InteropErrorKind` separates "could not parse" from "parsed but
        // structurally wrong", and so should the Python surface.
        let kind = match e.kind() {
            tpt_nrg_interop::InteropErrorKind::Parse => "json",
            tpt_nrg_interop::InteropErrorKind::InvalidSystem => "validation",
            other => other.as_str(),
        };
        fail(kind, e)
    })
}

/// Power-flow solver selection.
#[pyclass(eq, eq_int, from_py_object, name = "PowerFlowMethod")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyPowerFlowMethod {
    /// Full AC Newton-Raphson.
    #[pyo3(name = "NEWTON_RAPHSON")]
    NewtonRaphson,
    /// AC Gauss-Seidel.
    #[pyo3(name = "GAUSS_SEIDEL")]
    GaussSeidel,
    /// Fast decoupled (DC warm start plus Newton-Raphson refinement).
    #[pyo3(name = "FAST_DECOUPLED")]
    FastDecoupled,
    /// Linear DC power flow.
    #[pyo3(name = "DC")]
    DcPowerFlow,
}

impl From<PyPowerFlowMethod> for PowerFlowMethod {
    fn from(value: PyPowerFlowMethod) -> Self {
        match value {
            PyPowerFlowMethod::NewtonRaphson => Self::NewtonRaphson,
            PyPowerFlowMethod::GaussSeidel => Self::GaussSeidel,
            PyPowerFlowMethod::FastDecoupled => Self::FastDecoupled,
            PyPowerFlowMethod::DcPowerFlow => Self::DcPowerFlow,
        }
    }
}

#[pymethods]
impl PyPowerFlowMethod {
    /// The lower-case name accepted by the command line and `Format.parse`.
    #[getter]
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn name(&self) -> &'static str {
        match self {
            Self::NewtonRaphson => "newton-raphson",
            Self::GaussSeidel => "gauss-seidel",
            Self::FastDecoupled => "fast-decoupled",
            Self::DcPowerFlow => "dc",
        }
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn __repr__(&self) -> String {
        format!(
            "PowerFlowMethod.{}",
            self.name().to_uppercase().replace('-', "_")
        )
    }
}

#[pymodule]
fn tpt_nrg(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("EnergyError", m.py().get_type::<EnergyError>())?;
    m.add_class::<PyPowerFlowMethod>()?;
    m.add_class::<PyPowerFlowResult>()?;
    m.add_class::<PySystem>()?;
    m.add_class::<PyLcoeResult>()?;
    m.add_function(wrap_pyfunction!(formats, m)?)?;
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    Ok(())
}

/// List the exchange formats this build understands.
///
/// Returns
///
/// ```python
/// ['json', 'yaml', 'csv', 'matpower', 'psse', 'cim']
/// ```
#[pyfunction]
fn formats() -> Vec<&'static str> {
    vec!["json", "yaml", "csv", "matpower", "psse", "cim"]
}

/// Validate a system document without solving it.
///
/// Returns `None` when the document is valid, otherwise a string describing
/// the first problem found.
#[pyfunction]
fn validate(text: &str, format: &str) -> PyResult<Option<String>> {
    match parse(text, format) {
        Ok(_) => Ok(None),
        Err(e) => Python::attach(|py| {
            let value = e.value(py);
            let kind = value
                .getattr("kind")
                .and_then(|k| k.extract::<String>())
                .unwrap_or_else(|_| "validation".to_string());
            let message = value
                .getattr("message")
                .and_then(|m| m.extract::<String>())
                .unwrap_or_else(|_| e.to_string());
            Ok(Some(format!("{kind}: {message}")))
        }),
    }
}

/// A solved power flow.
#[pyclass(get_all, module = "tpt_nrg", skip_from_py_object, name = "PowerFlowResult")]
#[derive(Debug, Clone)]
pub struct PyPowerFlowResult {
    /// Whether the solver converged.
    converged: bool,
    /// Iteration count.
    iterations: usize,
    /// Per-bus voltage magnitude, in per-unit.
    voltage_magnitude_pu: Vec<f64>,
    /// Per-bus voltage angle, in radians.
    voltage_angle_rad: Vec<f64>,
    /// Total active-power losses, in MW.
    losses_mw: f64,
    /// Total reactive-power losses, in `MVAr`.
    losses_mvar: f64,
    /// Per-branch apparent flow, in MVA.
    branch_flow_mva: Vec<f64>,
    /// Per-branch loading, as a fraction of rating.
    branch_loading: Vec<f64>,
}

#[pymethods]
impl PyPowerFlowResult {
    /// Voltage and angle of bus `i`, as `(|V| pu, angle deg)`.
    fn bus(&self, i: usize) -> PyResult<(f64, f64)> {
        let v = *self
            .voltage_magnitude_pu
            .get(i)
            .ok_or_else(|| PyIndexError::new_err(format!("bus index {i} out of range")))?;
        let a = *self
            .voltage_angle_rad
            .get(i)
            .ok_or_else(|| PyIndexError::new_err(format!("bus index {i} out of range")))?;
        Ok((v, a.to_degrees()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PowerFlowResult(converged={}, iterations={}, losses_mw={:.3})",
            self.converged, self.iterations, self.losses_mw
        )
    }
}

/// A levelized-cost-of-energy result.
#[pyclass(get_all, module = "tpt_nrg", skip_from_py_object, name = "LcoeResult")]
#[derive(Debug, Clone)]
pub struct PyLcoeResult {
    /// Levelized cost in dollars per MWh.
    lcoe_dollar_per_mwh: f64,
    /// Capital recovery factor used, per year.
    capital_recovery_factor: f64,
}

#[pymethods]
impl PyLcoeResult {
    fn __repr__(&self) -> String {
        format!(
            "LcoeResult(lcoe_dollar_per_mwh={:.2})",
            self.lcoe_dollar_per_mwh
        )
    }
}

/// An energy system: the Python face of [`EnergySystem`].
#[pyclass(module = "tpt_nrg", skip_from_py_object, name = "System")]
#[derive(Debug, Clone)]
pub struct PySystem {
    inner: EnergySystem,
}

#[pymethods]
impl PySystem {
    /// Parse a system from `text` in the named format.
    ///
    /// `format` is one of `json`, `yaml`, `csv`, `matpower`, `psse`, `cim`.
    #[staticmethod]
    fn from_text(text: &str, format: &str) -> PyResult<Self> {
        parse(text, format).map(|inner| Self { inner })
    }

    /// Parse a system from a JSON document.
    #[staticmethod]
    fn from_json(text: &str) -> PyResult<Self> {
        parse(text, "json").map(|inner| Self { inner })
    }

    /// Read a system from a file, inferring the format from its extension.
    #[staticmethod]
    fn from_file(path: &str) -> PyResult<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| fail("io", format!("cannot read {path}: {e}")))?;
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("json")
            .to_ascii_lowercase();
        let format = match ext.as_str() {
            "yaml" | "yml" => "yaml",
            "csv" => "csv",
            "m" => "matpower",
            "raw" => "psse",
            "xml" | "rdf" | "cim" => "cim",
            "json" => "json",
            other => {
                return Err(fail(
                    "unsupported",
                    format!("cannot infer a format from extension `{other}`"),
                ))
            }
        };
        parse(&text, format).map(|inner| Self { inner })
    }

    /// The system identifier.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// The human-readable name.
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// Per-unit base in MVA.
    #[getter]
    fn base_mva(&self) -> f64 {
        self.inner.base_mva
    }

    /// Nominal frequency in Hz.
    #[getter]
    fn frequency_hz(&self) -> f64 {
        self.inner.frequency_hz
    }

    /// Number of buses.
    #[getter]
    fn bus_count(&self) -> usize {
        self.inner.buses.len()
    }

    /// Number of branches.
    #[getter]
    fn branch_count(&self) -> usize {
        self.inner.branches.len()
    }

    /// Number of generators.
    #[getter]
    fn generator_count(&self) -> usize {
        self.inner.generators.len()
    }

    /// Total scheduled load in MW.
    #[getter]
    fn total_load_mw(&self) -> f64 {
        self.inner.total_load_mw()
    }

    /// Solve a power flow.
    #[pyo3(signature = (method = None, *, tolerance = 1e-6, max_iterations = 50))]
    fn power_flow(
        &self,
        method: Option<PyPowerFlowMethod>,
        tolerance: f64,
        max_iterations: usize,
    ) -> PyResult<PyPowerFlowResult> {
        let method = method.unwrap_or(PyPowerFlowMethod::NewtonRaphson);
        let result = PowerFlowSolver::new(method.into())
            .with_tolerance(tolerance)
            .with_max_iterations(max_iterations)
            .solve(&self.inner)
            .map_err(|e| fail("non_convergence", e))?;
        Ok(PyPowerFlowResult {
            converged: result.converged,
            iterations: result.iterations,
            voltage_magnitude_pu: result.bus_voltage_magnitude_pu,
            voltage_angle_rad: result.bus_voltage_angle_rad,
            losses_mw: result.total_losses_mw,
            losses_mvar: result.total_losses_mvar,
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

    /// Run a lossless economic dispatch at `load_mw`.
    ///
    /// Returns `(generator_outputs_mw, marginal_cost_dollar_per_mwh,
    /// total_cost_dollar_per_h)`.
    fn economic_dispatch(&self, load_mw: f64) -> PyResult<(Vec<f64>, f64, f64)> {
        let r = economic_dispatch(&self.inner, load_mw).map_err(|e| fail("infeasible", e))?;
        Ok((
            r.generator_outputs_mw,
            r.marginal_cost_dollar_per_mwh,
            r.total_cost_dollar_per_h,
        ))
    }

    /// Run a priority-list unit commitment over `load_profile_mw`.
    ///
    /// Returns `(commitment, outputs_mw, total_cost_dollar)`, where
    /// `commitment[unit][interval]` is 1 when the unit is on.
    #[allow(clippy::needless_pass_by_value)]
    fn unit_commitment(
        &self,
        load_profile_mw: Vec<f64>,
    ) -> PyResult<(Vec<Vec<u8>>, Vec<Vec<f64>>, f64)> {
        if load_profile_mw.is_empty() {
            return Err(fail("validation", "load_profile_mw must not be empty"));
        }
        let r = unit_commitment(&self.inner, &load_profile_mw);
        Ok((r.commitment, r.outputs, r.total_cost_dollar))
    }

    /// Three-phase fault current at `bus`, in per-unit.
    fn fault_current_pu(&self, bus: usize) -> PyResult<f64> {
        if !self.inner.buses.iter().any(|b| b.id == bus) {
            return Err(fail("validation", format!("no bus with id {bus}")));
        }
        Ok(FaultAnalyzer::new(&self.inner)
            .calculate_fault_current(bus, FaultType::ThreePhase)
            .i_fault_pu)
    }

    /// Carbon intensity of the scheduled dispatch, in kg CO2 per MWh.
    fn carbon_intensity(&self) -> f64 {
        tpt_nrg_carbon::carbon_intensity(&self.inner)
    }

    /// Levelized cost of energy, in dollars per MWh.
    #[pyo3(signature = (capex_dollar, annual_energy_mwh, *, annual_fixed_om_dollar = 0.0,
        variable_om_dollar_per_mwh = 0.0, fuel_cost_dollar_per_mwh = 0.0,
        lifetime_years = 25, discount_rate = 0.07))]
    #[allow(clippy::too_many_arguments)]
    fn lcoe(
        &self,
        capex_dollar: f64,
        annual_energy_mwh: f64,
        annual_fixed_om_dollar: f64,
        variable_om_dollar_per_mwh: f64,
        fuel_cost_dollar_per_mwh: f64,
        lifetime_years: u32,
        discount_rate: f64,
    ) -> PyResult<PyLcoeResult> {
        if !annual_energy_mwh.is_finite() || annual_energy_mwh <= 0.0 {
            return Err(fail(
                "validation",
                "annual_energy_mwh must be a positive finite number",
            ));
        }
        let inputs = LcoeInputs {
            capex_dollar,
            annual_fixed_om_dollar,
            variable_om_dollar_per_mwh,
            fuel_cost_dollar_per_mwh,
            annual_energy_mwh,
            lifetime_years,
            discount_rate,
            capacity_factor: annual_energy_mwh / (self.inner.base_mva * 8760.0).max(1.0),
        };
        let lcoe = levelized_cost_of_energy(&inputs);
        Ok(PyLcoeResult {
            lcoe_dollar_per_mwh: lcoe,
            capital_recovery_factor: capital_recovery_factor(discount_rate, lifetime_years),
        })
    }

    /// Render a single-line diagram and heatmap as an SVG string.
    ///
    /// Pass the `result` of a previous `power_flow` to draw the solved case; omit
    /// it to draw the scheduled voltages instead.
    #[pyo3(signature = (result = None, *, show_legend = true, show_flow_labels = true))]
    #[allow(clippy::needless_pass_by_value)]
    fn to_svg(
        &self,
        result: Option<PyRef<'_, PyPowerFlowResult>>,
        show_legend: bool,
        show_flow_labels: bool,
    ) -> String {
        let options = tpt_nrg_viz::VizOptions {
            title: String::new(),
            show_bus_ids: true,
            show_flow_labels,
            show_legend,
            show_generators: true,
            background: Some("#ffffff".to_string()),
        };
        if result.is_none() {
            let solved = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson)
                .solve(&self.inner)
                .ok();
            return tpt_nrg_viz::render(&self.inner, solved.as_ref(), &options);
        }
        tpt_nrg_viz::render(&self.inner, None, &options)
    }

    /// Serialize to the named format.
    fn to_format(&self, format: &str) -> PyResult<String> {
        let format: tpt_nrg_interop::Format =
            tpt_nrg_interop::Format::parse(format).map_err(|e| fail("unsupported", e))?;
        tpt_nrg_interop::to_text(&self.inner, format).map_err(|e| fail("parse", e))
    }

    /// Serialize to pretty-printed JSON.
    fn to_json(&self) -> PyResult<String> {
        self.to_format("json")
    }

    /// Serialize to YAML.
    fn to_yaml(&self) -> PyResult<String> {
        self.to_format("yaml")
    }

    /// Serialize to a MATPOWER `case*.m` file.
    fn to_matpower(&self) -> PyResult<String> {
        self.to_format("matpower")
    }

    /// Serialize to a PSS/E RAW file.
    fn to_psse(&self) -> PyResult<String> {
        self.to_format("psse")
    }

    /// Serialize to CIM / IEC 61970 RDF/XML.
    fn to_cim(&self) -> PyResult<String> {
        self.to_format("cim")
    }

    /// A nested `dict` of the system, for inspection from Python.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let value = serde_json::to_value(&self.inner).map_err(|e| fail("parse", e))?;
        Ok(pythonize(value)?.into_bound(py))
    }

    fn __repr__(&self) -> String {
        format!(
            "System(id={:?}, name={:?}, buses={}, branches={})",
            self.inner.id,
            self.inner.name,
            self.inner.buses.len(),
            self.inner.branches.len()
        )
    }
}

/// Capital recovery factor, the annualized share of a capital cost.
fn capital_recovery_factor(discount_rate: f64, lifetime_years: u32) -> f64 {
    let n = f64::from(lifetime_years.max(1));
    if discount_rate.abs() < 1e-12 {
        return 1.0 / n;
    }
    let grown = (1.0 + discount_rate).powf(n);
    discount_rate * grown / (grown - 1.0)
}

/// Convert a `serde_json::Value` into the closest Python object.
fn pythonize(value: serde_json::Value) -> PyResult<Py<PyAny>> {
    Python::attach(|py| match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().into_any().unbind()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.into_any().unbind())
            } else {
                let f = n.as_f64().unwrap_or(f64::NAN);
                Ok(f.into_pyobject(py)?.into_any().unbind())
            }
        }
        serde_json::Value::String(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
        serde_json::Value::Array(items) => {
            let list = pyo3::types::PyList::empty(py);
            for item in items {
                list.append(pythonize(item)?)?;
            }
            Ok(list.into_any().unbind())
        }
        serde_json::Value::Object(fields) => {
            let dict = pyo3::types::PyDict::new(py);
            for (k, v) in fields {
                dict.set_item(k, pythonize(v)?)?;
            }
            Ok(dict.into_any().unbind())
        }
    })
}
