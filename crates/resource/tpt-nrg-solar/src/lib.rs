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

mod irradiance;
mod position;
mod pv;

pub use irradiance::{Irradiance, SkyCondition};
pub use position::{SolarModel, SolarPosition};
pub use pv::{PvPlant, PvPlantConfig};
