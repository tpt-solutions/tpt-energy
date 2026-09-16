//! Top-level [`EnergySystem`] container.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::branch::Branch;
use crate::bus::{Bus, BusType};
use crate::error::{CoreError, CoreResult};
use crate::generator::Generator;
use crate::load::Load;
use crate::storage::Storage;

/// Top-level container for a complete power-system model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergySystem {
    /// System identifier (e.g. `"ieee14"`).
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// Base apparent power for the per-unit system, in MVA.
    pub base_mva: f64,

    /// Nominal system frequency in Hz.
    pub frequency_hz: f64,

    /// Buses in the system.
    #[serde(default)]
    pub buses: Vec<Bus>,

    /// Branches in the system.
    #[serde(default)]
    pub branches: Vec<Branch>,

    /// Generators in the system.
    #[serde(default)]
    pub generators: Vec<Generator>,

    /// Loads in the system.
    #[serde(default)]
    pub loads: Vec<Load>,

    /// Storage units in the system.
    #[serde(default)]
    pub storage: Vec<Storage>,

    /// Free-form metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl EnergySystem {
    /// Construct a new system with the given id, name, base MVA, and
    /// frequency.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        base_mva: f64,
        frequency_hz: f64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            base_mva,
            frequency_hz,
            buses: Vec::new(),
            branches: Vec::new(),
            generators: Vec::new(),
            loads: Vec::new(),
            storage: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Deserialize an `EnergySystem` from a JSON string.
    pub fn from_json(s: &str) -> CoreResult<Self> {
        Ok(serde_json::from_str(s)?)
    }

    /// Deserialize an `EnergySystem` from a JSON file.
    pub fn from_json_file(path: impl AsRef<std::path::Path>) -> CoreResult<Self> {
        let s = std::fs::read_to_string(path)?;
        Self::from_json(&s)
    }

    /// Serialize to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> CoreResult<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Add a bus. Returns an error if a bus with the same id already exists.
    pub fn add_bus(&mut self, bus: Bus) -> CoreResult<()> {
        if self.bus_index(bus.id).is_some() {
            return Err(CoreError::DuplicateId(bus.id, "bus"));
        }
        self.buses.push(bus);
        Ok(())
    }

    /// Add a branch. Returns an error if either referenced bus is missing.
    pub fn add_branch(&mut self, branch: Branch) -> CoreResult<()> {
        if self.bus_index(branch.from_bus).is_none() {
            return Err(CoreError::UnknownBusForBranch {
                branch_id: branch.id,
                bus_id: branch.from_bus,
            });
        }
        if self.bus_index(branch.to_bus).is_none() {
            return Err(CoreError::UnknownBusForBranch {
                branch_id: branch.id,
                bus_id: branch.to_bus,
            });
        }
        if self.branch_index(branch.id).is_some() {
            return Err(CoreError::DuplicateId(branch.id, "branch"));
        }
        self.branches.push(branch);
        Ok(())
    }

    /// Add a generator. Returns an error if the referenced bus is missing.
    pub fn add_generator(&mut self, gen: Generator) -> CoreResult<()> {
        if self.bus_index(gen.bus_id).is_none() {
            return Err(CoreError::UnknownBusForGenerator {
                generator_id: gen.id,
                bus_id: gen.bus_id,
            });
        }
        if self.generator_index(gen.id).is_some() {
            return Err(CoreError::DuplicateId(gen.id, "generator"));
        }
        self.generators.push(gen);
        Ok(())
    }

    /// Add a load. Returns an error if the referenced bus is missing.
    pub fn add_load(&mut self, load: Load) -> CoreResult<()> {
        if self.bus_index(load.bus_id).is_none() {
            return Err(CoreError::UnknownBusForLoad {
                load_id: load.id,
                bus_id: load.bus_id,
            });
        }
        if self.load_index(load.id).is_some() {
            return Err(CoreError::DuplicateId(load.id, "load"));
        }
        self.loads.push(load);
        Ok(())
    }

    /// Add a storage unit. Returns an error if the referenced bus is missing.
    pub fn add_storage(&mut self, storage: Storage) -> CoreResult<()> {
        if self.bus_index(storage.bus_id).is_none() {
            return Err(CoreError::UnknownBusForStorage {
                storage_id: storage.id,
                bus_id: storage.bus_id,
            });
        }
        if self.storage_index(storage.id).is_some() {
            return Err(CoreError::DuplicateId(storage.id, "storage"));
        }
        self.storage.push(storage);
        Ok(())
    }

    /// Look up a bus by id.
    pub fn bus(&self, id: usize) -> CoreResult<&Bus> {
        self.buses
            .iter()
            .find(|b| b.id == id)
            .ok_or(CoreError::BusNotFound(id))
    }

    /// Look up a mutable bus by id.
    pub fn bus_mut(&mut self, id: usize) -> CoreResult<&mut Bus> {
        self.buses
            .iter_mut()
            .find(|b| b.id == id)
            .ok_or(CoreError::BusNotFound(id))
    }

    /// Total system active load in MW.
    #[must_use]
    pub fn total_load_mw(&self) -> f64 {
        self.buses.iter().map(|b| b.load_mw).sum::<f64>()
            + self
                .loads
                .iter()
                .filter(|l| l.in_service)
                .map(|l| l.p_mw)
                .sum::<f64>()
    }

    /// Total system reactive load in MVAr.
    #[must_use]
    pub fn total_load_mvar(&self) -> f64 {
        self.buses.iter().map(|b| b.load_mvar).sum::<f64>()
            + self
                .loads
                .iter()
                .filter(|l| l.in_service)
                .map(|l| l.q_mvar)
                .sum::<f64>()
    }

    /// Total installed generation capacity (MW) for in-service units.
    #[must_use]
    pub fn total_generation_capacity_mw(&self) -> f64 {
        self.generators
            .iter()
            .filter(|g| g.in_service)
            .map(|g| g.p_max_mw)
            .sum()
    }

    /// Number of slack buses (zero or one expected).
    #[must_use]
    pub fn slack_bus_count(&self) -> usize {
        self.buses
            .iter()
            .filter(|b| b.bus_type == BusType::Slack)
            .count()
    }

    /// Validate the system: returns `Ok(())` if there is exactly one slack bus
    /// and all references resolve.
    pub fn validate(&self) -> CoreResult<()> {
        // Branches
        for br in &self.branches {
            if self.bus_index(br.from_bus).is_none() {
                return Err(CoreError::UnknownBusForBranch {
                    branch_id: br.id,
                    bus_id: br.from_bus,
                });
            }
            if self.bus_index(br.to_bus).is_none() {
                return Err(CoreError::UnknownBusForBranch {
                    branch_id: br.id,
                    bus_id: br.to_bus,
                });
            }
        }
        // Generators
        for g in &self.generators {
            if self.bus_index(g.bus_id).is_none() {
                return Err(CoreError::UnknownBusForGenerator {
                    generator_id: g.id,
                    bus_id: g.bus_id,
                });
            }
        }
        // Loads
        for l in &self.loads {
            if self.bus_index(l.bus_id).is_none() {
                return Err(CoreError::UnknownBusForLoad {
                    load_id: l.id,
                    bus_id: l.bus_id,
                });
            }
        }
        // Storage
        for s in &self.storage {
            if self.bus_index(s.bus_id).is_none() {
                return Err(CoreError::UnknownBusForStorage {
                    storage_id: s.id,
                    bus_id: s.bus_id,
                });
            }
        }
        // Slack
        let n = self.slack_bus_count();
        if n == 0 {
            return Err(CoreError::NoSlackBus);
        }
        if n > 1 {
            return Err(CoreError::MultipleSlackBuses(n));
        }
        Ok(())
    }

    fn bus_index(&self, id: usize) -> Option<usize> {
        self.buses.iter().position(|b| b.id == id)
    }
    fn branch_index(&self, id: usize) -> Option<usize> {
        self.branches.iter().position(|b| b.id == id)
    }
    fn generator_index(&self, id: usize) -> Option<usize> {
        self.generators.iter().position(|g| g.id == id)
    }
    fn load_index(&self, id: usize) -> Option<usize> {
        self.loads.iter().position(|l| l.id == id)
    }
    fn storage_index(&self, id: usize) -> Option<usize> {
        self.storage.iter().position(|s| s.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::GeneratorType;

    fn simple_system() -> EnergySystem {
        let mut sys = EnergySystem::new("test", "Test", 100.0, 60.0);
        sys.add_bus(Bus::new(1, "B1", BusType::Slack).with_voltage_pu(1.06, 0.0))
            .unwrap();
        sys.add_bus(Bus::new(2, "B2", BusType::Pq).with_load(50.0, 20.0))
            .unwrap();
        sys.add_branch(Branch::new(1, "L1", 1, 2, 0.01, 0.05))
            .unwrap();
        sys.add_generator(
            Generator::new(1, "G1", GeneratorType::Thermal, 100.0, 10.0)
                .at_bus(1)
                .with_voltage_setpoint(1.06),
        )
        .unwrap();
        sys
    }

    #[test]
    fn construct_and_validate() {
        let sys = simple_system();
        sys.validate().unwrap();
        assert_eq!(sys.buses.len(), 2);
        assert_eq!(sys.branches.len(), 1);
        assert_eq!(sys.generators.len(), 1);
        assert!((sys.total_load_mw() - 50.0).abs() < 1e-12);
    }

    #[test]
    fn duplicate_bus_rejected() {
        let mut sys = EnergySystem::new("test", "Test", 100.0, 60.0);
        sys.add_bus(Bus::new(1, "B1", BusType::Slack)).unwrap();
        let r = sys.add_bus(Bus::new(1, "B1-dup", BusType::Pq));
        assert!(matches!(r, Err(CoreError::DuplicateId(1, "bus"))));
    }

    #[test]
    fn branch_missing_bus_rejected() {
        let mut sys = EnergySystem::new("test", "Test", 100.0, 60.0);
        sys.add_bus(Bus::new(1, "B1", BusType::Slack)).unwrap();
        let r = sys.add_branch(Branch::new(1, "L1", 1, 99, 0.01, 0.05));
        assert!(matches!(r, Err(CoreError::UnknownBusForBranch { .. })));
    }

    #[test]
    fn no_slack_rejected() {
        let mut sys = EnergySystem::new("test", "Test", 100.0, 60.0);
        sys.add_bus(Bus::new(1, "B1", BusType::Pq)).unwrap();
        let r = sys.validate();
        assert!(matches!(r, Err(CoreError::NoSlackBus)));
    }

    #[test]
    fn json_roundtrip() {
        let sys = simple_system();
        let s = sys.to_json_pretty().unwrap();
        let back = EnergySystem::from_json(&s).unwrap();
        assert_eq!(back.id, "test");
        assert_eq!(back.buses.len(), 2);
        assert!((back.base_mva - 100.0).abs() < 1e-12);
    }
}
