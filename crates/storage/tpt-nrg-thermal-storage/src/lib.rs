//! # tpt-nrg-thermal-storage
//!
//! Thermal energy storage with state-of-charge tracking.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// Thermal energy storage model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalStorage {
    /// Energy capacity in MWh_th.
    pub energy_capacity_mwh_th: f64,
    /// Charge/discharge power rating in MW_th.
    pub power_rating_mw: f64,
    /// Round-trip efficiency.
    pub round_trip_efficiency: f64,
    /// State of charge as a fraction (0-1).
    pub soc: f64,
    /// Minimum state of charge.
    pub min_soc: f64,
}

impl ThermalStorage {
    /// Construct a new thermal storage device.
    pub fn new(energy_capacity_mwh_th: f64, power_rating_mw: f64, round_trip_efficiency: f64) -> Self {
        Self {
            energy_capacity_mwh_th,
            power_rating_mw,
            round_trip_efficiency,
            soc: 0.5,
            min_soc: 0.0,
        }
    }

    /// Set the state-of-charge bounds.
    pub fn with_soc_bounds(mut self, min_soc: f64, initial_soc: f64) -> Self {
        self.min_soc = min_soc.clamp(0.0, 1.0);
        self.soc = initial_soc.clamp(self.min_soc, 1.0);
        self
    }

    /// Charge: add `power_mw` for `duration_h`. Returns energy stored.
    pub fn charge(&mut self, power_mw: f64, duration_h: f64) -> f64 {
        let power = power_mw.max(0.0).min(self.power_rating_mw);
        let headroom = (1.0 - self.soc) * self.energy_capacity_mwh_th;
        let in_energy = power * duration_h;
        let stored = in_energy * self.round_trip_efficiency.sqrt();
        let actual = stored.min(headroom).max(0.0);
        self.soc += actual / self.energy_capacity_mwh_th;
        actual
    }

    /// Discharge: deliver `power_mw` for `duration_h`. Returns energy out.
    pub fn discharge(&mut self, power_mw: f64, duration_h: f64) -> f64 {
        let power = power_mw.max(0.0).min(self.power_rating_mw);
        let available = (self.soc - self.min_soc) * self.energy_capacity_mwh_th;
        let out_request = power * duration_h;
        let actual = out_request.min(available).max(0.0);
        let stored_needed = actual / self.round_trip_efficiency.sqrt();
        self.soc -= stored_needed / self.energy_capacity_mwh_th;
        actual
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charge_discharge_round_trip() {
        let mut t = ThermalStorage::new(100.0, 50.0, 0.8).with_soc_bounds(0.0, 0.5);
        // Charge 50 MWh for 1 h.
        let stored = t.charge(50.0, 1.0);
        // 50 * sqrt(0.8) = 44.72 MWh stored
        assert!((stored - 44.72).abs() < 0.01);
        assert!((t.soc - 0.9472).abs() < 1e-3);
    }

    #[test]
    fn discharge_limited_by_soc() {
        let mut t = ThermalStorage::new(100.0, 50.0, 0.8).with_soc_bounds(0.1, 0.3);
        // Available: (0.3-0.1)*100 = 20 MWh
        // Discharge 50 MW for 1h = 50 MWh requested; but limited to 20.
        let out = t.discharge(50.0, 1.0);
        assert!((out - 20.0).abs() < 1e-6);
    }
}
