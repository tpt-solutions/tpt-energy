//! # tpt-nrg-hydrogen
//!
//! Hydrogen storage: electrolyzer (electricity → H₂) and fuel cell
//! (H₂ → electricity).

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// Electrolyzer model: electricity in, hydrogen out.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Electrolyzer {
    /// Specific energy consumption in kWh per kg of H₂ produced (typical
    /// 50-55 kWh/kg for modern PEM).
    pub specific_energy_kwh_per_kg: f64,
    /// Maximum input power in MW.
    pub power_rating_mw: f64,
}

impl Electrolyzer {
    /// Construct a new electrolyzer.
    pub fn new(specific_energy_kwh_per_kg: f64, power_rating_mw: f64) -> Self {
        Self {
            specific_energy_kwh_per_kg,
            power_rating_mw,
        }
    }

    /// Produce hydrogen (kg) from `energy_mwh` of electricity.
    pub fn produce_hydrogen(&self, energy_mwh: f64) -> f64 {
        if self.specific_energy_kwh_per_kg <= 0.0 {
            return 0.0;
        }
        let energy_kwh = energy_mwh * 1000.0;
        energy_kwh / self.specific_energy_kwh_per_kg
    }
}

/// Fuel cell model: hydrogen in, electricity out.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelCell {
    /// Specific energy output in kWh per kg of H₂ (typical 16-20 kWh/kg
    /// for PEMFC at system level).
    pub specific_energy_kwh_per_kg: f64,
    /// Maximum output power in MW.
    pub power_rating_mw: f64,
}

impl FuelCell {
    /// Construct a new fuel cell.
    pub fn new(specific_energy_kwh_per_kg: f64, power_rating_mw: f64) -> Self {
        Self {
            specific_energy_kwh_per_kg,
            power_rating_mw,
        }
    }

    /// Generate electricity (MWh) from `hydrogen_kg` of H₂.
    pub fn generate_electricity(&self, hydrogen_kg: f64) -> f64 {
        (self.specific_energy_kwh_per_kg * hydrogen_kg) / 1000.0
    }
}

/// A complete hydrogen storage system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydrogenSystem {
    /// Electrolyzer.
    pub electrolyzer: Electrolyzer,
    /// Fuel cell.
    pub fuel_cell: FuelCell,
    /// H₂ tank capacity in kg.
    pub tank_capacity_kg: f64,
    /// Current H₂ mass in the tank (kg).
    pub current_hydrogen_kg: f64,
}

impl HydrogenSystem {
    /// Construct a new hydrogen system.
    pub fn new(electrolyzer: Electrolyzer, fuel_cell: FuelCell, tank_capacity_kg: f64) -> Self {
        Self {
            electrolyzer,
            fuel_cell,
            tank_capacity_kg,
            current_hydrogen_kg: 0.0,
        }
    }

    /// Charge the H₂ tank with `energy_mwh` of electricity. Returns H₂
    /// mass produced (kg), capped by remaining tank capacity.
    pub fn charge(&mut self, energy_mwh: f64) -> f64 {
        let produced = self.electrolyzer.produce_hydrogen(energy_mwh);
        let headroom = self.tank_capacity_kg - self.current_hydrogen_kg;
        let actual = produced.min(headroom).max(0.0);
        self.current_hydrogen_kg += actual;
        actual
    }

    /// Discharge the H₂ tank to produce electricity up to `power_mw` for
    /// `duration_h` hours. Returns the electricity actually generated.
    pub fn discharge(&mut self, power_mw: f64, duration_h: f64) -> f64 {
        let power = power_mw.min(self.fuel_cell.power_rating_mw).max(0.0);
        let requested = power * duration_h;
        // MWh needed = requested / (specific_energy_kwh_per_kg / 1000) / 1000
        // = requested * 1000 / specific_energy_kwh_per_kg kg of H2.
        let kg_needed = (requested * 1000.0) / self.fuel_cell.specific_energy_kwh_per_kg;
        let kg_available = self.current_hydrogen_kg;
        let kg_used = kg_needed.min(kg_available);
        let out = self.fuel_cell.generate_electricity(kg_used);
        self.current_hydrogen_kg -= kg_used;
        out
    }

    /// Round-trip efficiency (electricity → H₂ → electricity) as a
    /// fraction, assuming ideal tank.
    pub fn round_trip_efficiency(&self) -> f64 {
        let charge_kwh = 1.0;
        let h2_kg = self.electrolyzer.produce_hydrogen(charge_kwh / 1000.0);
        let out_kwh = self.fuel_cell.generate_electricity(h2_kg) * 1000.0;
        out_kwh / charge_kwh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn electrolyzer_produces_h2() {
        let e = Electrolyzer::new(50.0, 10.0);
        // 1 MWh at 50 kWh/kg = 1000/50 = 20 kg
        assert!((e.produce_hydrogen(1.0) - 20.0).abs() < 1e-9);
    }

    #[test]
    fn fuel_cell_generates_power() {
        let f = FuelCell::new(17.0, 5.0);
        // 10 kg at 17 kWh/kg = 170 kWh = 0.17 MWh
        assert!((f.generate_electricity(10.0) - 0.17).abs() < 1e-9);
    }

    #[test]
    fn system_charge_and_discharge() {
        let mut sys = HydrogenSystem::new(
            Electrolyzer::new(50.0, 10.0),
            FuelCell::new(17.0, 5.0),
            100.0,
        );
        let h2 = sys.charge(1.0);
        assert!((h2 - 20.0).abs() < 1e-9);
        assert!((sys.current_hydrogen_kg - 20.0).abs() < 1e-9);
        let out = sys.discharge(5.0, 0.1);
        // 0.1h at 5 MW = 0.5 MWh requested. H2 needed = 0.5*1000/17 = 29.4 kg.
        // Only 20 kg available, so 20*17/1000 = 0.34 MWh delivered.
        assert!((out - 0.34).abs() < 1e-3, "out = {out}");
    }

    #[test]
    fn round_trip_around_30_percent() {
        let sys = HydrogenSystem::new(
            Electrolyzer::new(50.0, 10.0),
            FuelCell::new(17.0, 5.0),
            100.0,
        );
        let eff = sys.round_trip_efficiency();
        // 17/50 = 34%
        assert!((eff - 0.34).abs() < 1e-3, "eff = {eff}");
    }
}
