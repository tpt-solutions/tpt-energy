//! # tpt-nrg-core
//!
//! Core data model for the TPT Energy power-systems toolkit.
//!
//! This crate defines the canonical in-memory representation of an electrical
//! energy system: buses, branches, generators, loads, storage devices, and the
//! top-level [`EnergySystem`] container. It also provides JSON (de)serialization
//! so that test cases (IEEE 14/30/57/118-bus, custom scenarios) can be loaded
//! from disk.
//!
//! ## Example
//!
//! ```rust
//! use tpt_nrg_core::{EnergySystem, Bus, BusType, Branch, Generator, GeneratorType};
//!
//! let mut sys = EnergySystem::new("ieee14", "IEEE 14-Bus", 100.0, 60.0);
//! let bus1 = Bus::new(1, "Bus 1", BusType::Slack)
//!     .with_voltage_pu(1.06, 0.0);
//! sys.add_bus(bus1);
//!
//! let gen = Generator::new(1, "G1", GeneratorType::Thermal, 100.0, 50.0);
//! sys.add_generator(gen);
//! ```

#![deny(missing_docs)]

mod branch;
mod bus;
mod curves;
mod energy_system;
mod error;
mod generator;
mod load;
mod storage;

pub use branch::Branch;
pub use bus::{Bus, BusType};
pub use curves::{CostCurve, CostSegment, HeatRateCurve, PowerCurve};
pub use energy_system::EnergySystem;
pub use error::{CoreError, CoreResult};
pub use generator::{Generator, GeneratorType};
pub use load::Load;
pub use storage::Storage;
