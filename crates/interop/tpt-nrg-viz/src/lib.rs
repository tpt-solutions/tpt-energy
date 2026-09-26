//! # tpt-nrg-viz
//!
//! Dependency-free SVG rendering for TPT Energy power-flow results: a
//! single-line diagram plus a voltage and branch-loading heatmap.
//!
//! Everything is plain `String` output, so the result can be written to a
//! file, served to a browser, or inlined into a report without any template
//! engine or asset pipeline.
//!
//! ## Example
//!
//! ```rust
//! use tpt_nrg_core::EnergySystem;
//! use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};
//! use tpt_nrg_viz::{render, VizOptions};
//!
//! let system = EnergySystem::from_json(include_str!("../../../../test-data/ieee/ieee14.json"))?;
//! let result = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson).solve(&system)?;
//! let svg = render(&system, Some(&result), &VizOptions::default());
//! assert!(svg.starts_with("<svg"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![deny(missing_docs)]

mod heatmap;
pub mod layout;
mod sld;

pub use heatmap::{loading_band, voltage_band, Band, LoadingBand};
pub use layout::{Layout, Point};
pub use sld::{render, VizOptions};
