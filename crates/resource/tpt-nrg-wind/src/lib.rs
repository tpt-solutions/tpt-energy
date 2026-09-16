//! # tpt-nrg-wind
//!
//! Wind power modelling: vertical wind speed extrapolation (log / power-law),
//! Weibull probability, wake-loss models (Jensen/PARK, Frandsen, simple
//! eddy-viscosity), and farm-level output.

#![deny(missing_docs)]

#[cfg(feature = "substrate")]
mod substrate;
mod turbine;
mod wake;
mod wind;

#[cfg(feature = "substrate")]
pub use substrate::normal_sample_mean_var;
pub use turbine::{WindModel, WindTurbine};
pub use wake::{WakeModel, WindFarm};
