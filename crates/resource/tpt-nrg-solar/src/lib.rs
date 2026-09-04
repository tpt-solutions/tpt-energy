//! # tpt-nrg-solar
//!
//! Solar position (SPA-equivalent), clear-sky irradiance, plane-of-array
//! transposition, and PV plant output with temperature derating.
//!
//! This crate is a self-contained implementation of the algorithms. The
//! upstream `tpt-science::astronomy` SPA can be swapped in via the
//! `tpt-substrate` feature when the registry is available (see
//! `todo.md` Phase 8).

#![deny(missing_docs)]

#[cfg(feature = "substrate")]
mod substrate;
mod irradiance;
mod position;
mod pv;

pub use irradiance::{Irradiance, SkyCondition};
pub use position::{SolarModel, SolarPosition};
pub use pv::{PvPlant, PvPlantConfig};
#[cfg(feature = "substrate")]
pub use substrate::{earth_distance_correction_ghi, earth_sun_distance_au};
