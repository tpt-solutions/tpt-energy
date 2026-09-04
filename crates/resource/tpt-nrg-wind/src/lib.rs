//! # tpt-nrg-wind
//!
//! Wind power modelling: vertical wind speed extrapolation (log / power-law),
//! Weibull probability, wake-loss models (Jensen/PARK, Frandsen, simple
//! eddy-viscosity), and farm-level output.

#![deny(missing_docs)]

mod turbine;
#[cfg(feature = "substrate")]
mod substrate;
mod wake;
mod wind;

pub use turbine::{WindModel, WindTurbine};
pub use wake::{WakeModel, WindFarm};
#[cfg(feature = "substrate")]
pub use substrate::{normal_sample_mean_var};
