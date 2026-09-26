//! # tpt-nrg-interop
//!
//! Import and export [`EnergySystem`] models in the formats power-systems
//! engineers actually exchange:
//!
//! - **MATPOWER** (two-way) — `case*.m` MATLAB scripts.
//! - **PSS/E** (two-way) — RAW data files.
//! - **CIM / IEC 61970** (two-way) — RDF/XML subset.
//! - **YAML** (import / export) — the same shape as the native JSON.
//! - **CSV** (import / export) — one table per record type.
//!
//! Every converter validates its output with
//! [`EnergySystem::validate`], so a successful return always means a
//! structurally sound system.
//!
//! ## Example
//!
//! ```rust
//! use tpt_nrg_core::EnergySystem;
//! use tpt_nrg_interop::matpower;
//!
//! let case = "mpc.baseMVA = 100;\nmpc.bus = [\n 1 3 0 0 0 0 1 1.06 0 132 1 1.1 0.9;\n 2 1 0 0 0 0 1 1.0 0 132 1 1.1 0.9;\n];\nmpc.gen = [\n 1 0 0 0 0 1.06 100 1 100 0;\n];\nmpc.branch = [\n 1 2 0.01 0.05 0 0 0 0 0 0 1;\n];";
//! let system: EnergySystem = matpower::from_matpower(case)?;
//! assert_eq!(system.buses.len(), 2);
//!
//! // ... and back again.
//! let text = matpower::to_matpower(&system)?;
//! assert!(text.contains("mpc.baseMVA = 100;"));
//! # Ok::<(), tpt_nrg_interop::InteropError>(())
//! ```

#![deny(missing_docs)]

pub mod cim;
pub mod diff;
mod error;
pub mod matpower;
pub mod psse;
pub mod tabular;

pub use diff::{
    diff_systems, diff_systems_with_tolerance, diff_text, diff_text_with_tolerance, round_trip,
    Difference, DifferenceKind,
};
pub use error::{InteropError, InteropErrorKind, InteropResult};
pub use tpt_nrg_core::EnergySystem;

/// A supported exchange format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// Native TPT Energy JSON.
    Json,
    /// TPT Energy YAML (same schema as the JSON).
    Yaml,
    /// Flat CSV table with a `record` column.
    Csv,
    /// MATPOWER `case*.m` MATLAB script.
    Matpower,
    /// PSS/E RAW data file.
    Psse,
    /// CIM / IEC 61970 RDF/XML.
    Cim,
}

impl Format {
    /// Parse a format name or file extension (`mpc`/`matpower`, `raw`/`psse`,
    /// `rdf`/`xml`/`cim`, `yml`/`yaml`, `csv`, `json`).
    ///
    /// # Errors
    ///
    /// Returns [`InteropError::Unsupported`] if the name is not recognised.
    pub fn parse(name: &str) -> InteropResult<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "yaml" | "yml" => Ok(Self::Yaml),
            "csv" => Ok(Self::Csv),
            "matpower" | "m" | "mpc" => Ok(Self::Matpower),
            "psse" | "raw" => Ok(Self::Psse),
            "cim" | "rdf" | "xml" => Ok(Self::Cim),
            other => Err(InteropError::unsupported(format!(
                "format `{other}`; expected one of json, yaml, csv, matpower, psse, cim"
            ))),
        }
    }

    /// Infer the format from a file extension, so `case.m`, `case.raw`,
    /// `case.rdf`, `case.yaml`, `case.csv`, and `case.json` can each be
    /// detected without the caller repeating the format on the command line.
    ///
    /// # Errors
    ///
    /// Returns [`InteropError::Unsupported`] if the path has no extension, or
    /// [`InteropError::Unsupported`] from [`Format::parse`] if the extension
    /// is not a supported format.
    pub fn from_path(path: impl AsRef<std::path::Path>) -> InteropResult<Self> {
        let path = path.as_ref();
        let extension = path.extension().and_then(|e| e.to_str()).ok_or_else(|| {
            InteropError::unsupported(format!(
                "no file extension in `{}`; pass the format explicitly",
                path.display()
            ))
        })?;
        Self::parse(extension)
    }

    /// Canonical lower-case name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Csv => "csv",
            Self::Matpower => "matpower",
            Self::Psse => "psse",
            Self::Cim => "cim",
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Parse an [`EnergySystem`] from `text` in the given format.
///
/// # Errors
///
/// Returns [`InteropError::Parse`] if the text is malformed in `format`,
/// [`InteropError::InvalidSystem`] if it parses but is not a valid system, and
/// [`InteropError::Unsupported`] for an unrecognised format.
pub fn from_text(text: &str, format: Format) -> InteropResult<EnergySystem> {
    match format {
        // `EnergySystem::from_json` collapses parse and validation failures
        // into `CoreError`, so they are separated here to keep `kind`
        // meaningful for callers that branch on it.
        Format::Json => {
            let system: EnergySystem = serde_json::from_str(text)
                .map_err(|e| InteropError::parse("json", e.to_string()))?;
            system.validate().map_err(InteropError::InvalidSystem)?;
            Ok(system)
        }
        Format::Yaml => tabular::from_yaml(text),
        Format::Csv => tabular::from_flat_csv(text),
        Format::Matpower => matpower::from_matpower(text),
        Format::Psse => psse::from_psse(text),
        Format::Cim => cim::from_cim(text),
    }
}

/// Serialize an [`EnergySystem`] to `text` in the given format.
///
/// # Errors
///
/// Returns [`InteropError::Parse`] if the system cannot be represented in
/// `format`.
pub fn to_text(system: &EnergySystem, format: Format) -> InteropResult<String> {
    match format {
        Format::Json => system
            .to_json_pretty()
            .map_err(|e| InteropError::parse("json", e.to_string())),
        Format::Yaml => tabular::to_yaml(system),
        Format::Csv => tabular::to_flat_csv(system),
        Format::Matpower => matpower::to_matpower(system),
        Format::Psse => psse::to_psse(system),
        Format::Cim => cim::to_cim(system),
    }
}
