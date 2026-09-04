//! Bus definition.

use serde::{Deserialize, Serialize};

/// Classification of an electrical bus in a power-flow model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum BusType {
    /// Slack (swing) bus: absorbs the system imbalance; voltage magnitude and
    /// angle are fixed.
    Slack,
    /// PV (generator) bus: voltage magnitude and active power are fixed;
    /// reactive power and angle are computed.
    Pv,
    /// PQ (load) bus: active and reactive power are fixed; voltage magnitude
    /// and angle are computed.
    Pq,
    /// Bus that is not electrically connected to any other.
    Isolated,
}

impl Default for BusType {
    fn default() -> Self {
        Self::Pq
    }
}

/// A single electrical bus in an [`EnergySystem`](crate::EnergySystem).
///
/// Voltages are in per-unit on the system base (see
/// [`EnergySystem::base_mva`](crate::EnergySystem::base_mva)).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bus {
    /// Unique numeric bus identifier.
    pub id: usize,

    /// Human-readable bus name.
    pub name: String,

    /// Bus type.
    #[serde(rename = "type")]
    pub bus_type: BusType,

    /// Voltage magnitude schedule in per-unit.
    #[serde(default = "default_voltage_magnitude")]
    pub voltage_magnitude_pu: f64,

    /// Voltage angle schedule in radians.
    #[serde(default)]
    pub voltage_angle_rad: f64,

    /// Base voltage in kV (for reporting only).
    #[serde(default = "default_base_kv")]
    pub base_kv: f64,

    /// Active power load in MW (positive = consumption).
    #[serde(default)]
    pub load_mw: f64,

    /// Reactive power load in MVAr (positive = consumption).
    #[serde(default)]
    pub load_mvar: f64,

    /// Active power generation schedule in MW.
    #[serde(default)]
    pub generation_mw: f64,

    /// Reactive power generation schedule in MVAr.
    #[serde(default)]
    pub generation_mvar: f64,

    /// Conductance (G) of shunt element, in per-unit.
    #[serde(default)]
    pub shunt_conductance_pu: f64,

    /// Susceptance (B) of shunt element, in per-unit.
    #[serde(default)]
    pub shunt_susceptance_pu: f64,

    /// Geographic area / zone identifier (for reporting only).
    #[serde(default)]
    pub area: Option<String>,
}

fn default_voltage_magnitude() -> f64 {
    1.0
}

fn default_base_kv() -> f64 {
    110.0
}

impl Bus {
    /// Construct a new bus with the given id, name, and type. Default values
    /// are used for the remaining fields.
    pub fn new(id: usize, name: impl Into<String>, bus_type: BusType) -> Self {
        Self {
            id,
            name: name.into(),
            bus_type,
            voltage_magnitude_pu: match bus_type {
                BusType::Slack | BusType::Pv => 1.0,
                _ => 1.0,
            },
            voltage_angle_rad: 0.0,
            base_kv: default_base_kv(),
            load_mw: 0.0,
            load_mvar: 0.0,
            generation_mw: 0.0,
            generation_mvar: 0.0,
            shunt_conductance_pu: 0.0,
            shunt_susceptance_pu: 0.0,
            area: None,
        }
    }

    /// Set the voltage magnitude (pu) and angle (radians) schedule.
    pub fn with_voltage_pu(mut self, voltage_magnitude_pu: f64, voltage_angle_rad: f64) -> Self {
        self.voltage_magnitude_pu = voltage_magnitude_pu;
        self.voltage_angle_rad = voltage_angle_rad;
        self
    }

    /// Set the base voltage in kV.
    pub fn with_base_kv(mut self, base_kv: f64) -> Self {
        self.base_kv = base_kv;
        self
    }

    /// Set the load in MW and MVAr.
    pub fn with_load(mut self, load_mw: f64, load_mvar: f64) -> Self {
        self.load_mw = load_mw;
        self.load_mvar = load_mvar;
        self
    }

    /// Set the scheduled generation in MW and MVAr.
    pub fn with_generation(mut self, generation_mw: f64, generation_mvar: f64) -> Self {
        self.generation_mw = generation_mw;
        self.generation_mvar = generation_mvar;
        self
    }

    /// Set the shunt admittance in per-unit (G + jB).
    pub fn with_shunt(mut self, g_pu: f64, b_pu: f64) -> Self {
        self.shunt_conductance_pu = g_pu;
        self.shunt_susceptance_pu = b_pu;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_construction_defaults() {
        let b = Bus::new(1, "B1", BusType::Pq);
        assert_eq!(b.id, 1);
        assert_eq!(b.bus_type, BusType::Pq);
        assert!((b.voltage_magnitude_pu - 1.0).abs() < 1e-12);
        assert!((b.voltage_angle_rad - 0.0).abs() < 1e-12);
    }

    #[test]
    fn bus_with_voltage() {
        let b = Bus::new(1, "Slack", BusType::Slack).with_voltage_pu(1.06, 0.0);
        assert!((b.voltage_magnitude_pu - 1.06).abs() < 1e-12);
    }

    #[test]
    fn bus_serde_roundtrip() {
        let b = Bus::new(1, "B1", BusType::Slack)
            .with_voltage_pu(1.05, 0.0)
            .with_load(20.0, 5.0);
        let s = serde_json::to_string(&b).unwrap();
        let d: Bus = serde_json::from_str(&s).unwrap();
        assert_eq!(d.id, b.id);
        assert_eq!(d.bus_type, b.bus_type);
        assert!((d.load_mw - 20.0).abs() < 1e-12);
    }
}
