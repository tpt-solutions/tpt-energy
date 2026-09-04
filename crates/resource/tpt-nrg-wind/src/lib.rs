//! # tpt-nrg-wind
//!
//! Wind power modelling: vertical wind speed extrapolation (log / power-law),
//! Weibull probability, wake-loss models (Jensen/PARK, Frandsen, simple
//! eddy-viscosity), and farm-level output.

#![deny(missing_docs)]

mod turbine;
mod wake;
mod wind;

pub use turbine::{WindModel, WindTurbine};
pub use wake::{WakeModel, WindFarm};
