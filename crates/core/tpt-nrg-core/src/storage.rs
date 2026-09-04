//! Storage unit definition.

use serde::{Deserialize, Serialize};

/// A storage device attached to a bus (battery, hydrogen, thermal, pumped
/// hydro).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storage {
    /// Unique numeric storage id.
    pub id: usize,

    /// Human-readable name.
    pub name: String,

    /// Bus id where the storage is connected.
    pub bus_id: usize,

    /// Energy capacity in MWh.
    pub energy_capacity_mwh: f64,

    /// Rated power in MW (same value used for charge and discharge limits
    /// unless overridden).
    pub power_rating_mw: f64,

    /// Initial state of charge as a fraction in `[0, 1]`.
    #[serde(default = "default_soc")]
    pub initial_soc: f64,

    /// Round-trip efficiency in `(0, 1]`.
    #[serde(default = "default_eff")]
    pub round_trip_efficiency: f64,

    /// Whether the storage is in service.
    #[serde(default = "default_in_service")]
    pub in_service: bool,
}

fn default_soc() -> f64 {
    0.5
}
fn default_eff() -> f64 {
    0.9
}
fn default_in_service() -> bool {
    true
}

impl Storage {
    /// Construct a new storage device.
    pub fn new(
        id: usize,
        name: impl Into<String>,
        bus_id: usize,
        energy_capacity_mwh: f64,
        power_rating_mw: f64,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            bus_id,
            energy_capacity_mwh,
            power_rating_mw,
            initial_soc: default_soc(),
            round_trip_efficiency: default_eff(),
            in_service: true,
        }
    }

    /// Set the initial state of charge.
    pub fn with_initial_soc(mut self, soc: f64) -> Self {
        self.initial_soc = soc.clamp(0.0, 1.0);
        self
    }

    /// Set the round-trip efficiency.
    pub fn with_round_trip_efficiency(mut self, eff: f64) -> Self {
        self.round_trip_efficiency = eff.clamp(0.0, 1.0);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_construction() {
        let s = Storage::new(1, "B1", 2, 100.0, 50.0);
        assert_eq!(s.bus_id, 2);
        assert!((s.energy_capacity_mwh - 100.0).abs() < 1e-12);
        assert!((s.round_trip_efficiency - 0.9).abs() < 1e-12);
    }
}
