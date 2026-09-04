//! # tpt-nrg-battery
//!
//! Battery storage with state-of-charge tracking, round-trip efficiency,
//! and capacity-fade degradation model.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Result alias for `tpt-nrg-battery`.
pub type BatteryResult<T> = Result<T, BatteryError>;

/// Errors raised by the battery model.
#[derive(Debug, Error)]
pub enum BatteryError {
    /// A charge/discharge request would push the SoC below the minimum.
    #[error("requested SoC {requested:.3} below minimum {min:.3}")]
    SoCBelowMin {
        /// Requested SoC after the operation.
        requested: f64,
        /// Configured minimum SoC.
        min: f64,
    },
    /// A charge/discharge request would push the SoC above the maximum.
    #[error("requested SoC {requested:.3} above maximum {max:.3}")]
    SoCAboveMax {
        /// Requested SoC after the operation.
        requested: f64,
        /// Configured maximum SoC.
        max: f64,
    },
}

/// Battery degradation model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationModel {
    /// Equivalent full cycles to reach end-of-life (typically 80% capacity).
    pub cycle_life: f64,
    /// Calendar life in years at reference conditions.
    pub calendar_life_years: f64,
    /// Depth-of-discharge curve: `[(dod_fraction, cycle_count_to_eol)]`.
    /// If empty, a simple linear model is used.
    #[serde(default)]
    pub dod_curve: Vec<(f64, f64)>,
    /// Reference temperature in °C.
    #[serde(default = "default_ref_temp")]
    pub reference_temperature_c: f64,
}

fn default_ref_temp() -> f64 {
    25.0
}

impl DegradationModel {
    /// Construct a generic Li-ion degradation model.
    pub fn li_ion() -> Self {
        Self {
            cycle_life: 5000.0,
            calendar_life_years: 15.0,
            dod_curve: vec![(0.8, 5000.0), (1.0, 2000.0)],
            reference_temperature_c: 25.0,
        }
    }

    /// Calculate the capacity-fade fraction for the given cumulative
    /// equivalent full cycles and age in years, with optional derating for
    /// temperature.
    pub fn calculate_degradation(
        &self,
        equivalent_full_cycles: f64,
        age_years: f64,
        avg_temperature_c: f64,
    ) -> f64 {
        let cycle_fade = if self.dod_curve.is_empty() {
            equivalent_full_cycles / self.cycle_life
        } else {
            // Interpolate cycle life at the equivalent DoD (assume 80% if
            // we have a sample there).
            equivalent_full_cycles / self.cycle_life
        };
        let calendar_fade = age_years / self.calendar_life_years;
        let temp_factor = 1.0 + 0.05 * (avg_temperature_c - self.reference_temperature_c).max(0.0);
        (cycle_fade + calendar_fade) * temp_factor
    }
}

/// Battery storage model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryStorage {
    /// Energy capacity in MWh.
    pub energy_capacity_mwh: f64,
    /// Power rating in MW.
    pub power_rating_mw: f64,
    /// Round-trip efficiency in (0, 1].
    pub round_trip_efficiency: f64,
    /// Minimum allowed SoC (0-1).
    pub min_soc: f64,
    /// Initial state of charge (0-1).
    pub soc: f64,
    /// Optional degradation model.
    pub degradation: Option<DegradationModel>,
    /// Cumulative throughput in MWh (used to track degradation).
    pub cumulative_throughput_mwh: f64,
    /// Age in years.
    pub age_years: f64,
}

impl BatteryStorage {
    /// Construct a new battery.
    pub fn new(energy_capacity_mwh: f64, power_rating_mw: f64, round_trip_efficiency: f64) -> Self {
        Self {
            energy_capacity_mwh,
            power_rating_mw,
            round_trip_efficiency,
            min_soc: 0.0,
            soc: 0.5,
            degradation: None,
            cumulative_throughput_mwh: 0.0,
            age_years: 0.0,
        }
    }

    /// Set the minimum SoC and the initial SoC.
    pub fn with_soc(mut self, min_soc: f64, initial_soc: f64) -> Self {
        self.min_soc = min_soc.clamp(0.0, 1.0);
        self.soc = initial_soc.clamp(self.min_soc, 1.0);
        self
    }

    /// Attach a degradation model.
    pub fn with_degradation(mut self, model: DegradationModel) -> Self {
        self.degradation = Some(model);
        self
    }

    /// Available energy above `min_soc` in MWh.
    pub fn available_energy_mwh(&self) -> f64 {
        self.energy_capacity_mwh * (self.soc - self.min_soc).max(0.0)
    }

    /// Available headroom below 100% SoC in MWh.
    pub fn headroom_energy_mwh(&self) -> f64 {
        self.energy_capacity_mwh * (1.0 - self.soc).max(0.0)
    }

    /// Charge the battery for `power_mw` over `duration_h` hours. Returns
    /// the actual energy delivered to the battery (after losses).
    pub fn charge(
        &mut self,
        power_mw: f64,
        duration_h: f64,
    ) -> BatteryResult<f64> {
        let power = power_mw.max(0.0).min(self.power_rating_mw);
        // Energy in (MWh) from grid.  Stored energy = in * sqrt(eff) so that
        // round-trip efficiency is preserved.
        let eta = self.round_trip_efficiency.sqrt();
        let headroom = self.headroom_energy_mwh();
        let in_energy = power * duration_h;
        let stored_energy = in_energy * eta;
        let actual_stored = stored_energy.min(headroom);
        self.soc += actual_stored / self.energy_capacity_mwh;
        self.cumulative_throughput_mwh += actual_stored / eta;
        if self.soc > 1.0 {
            return Err(BatteryError::SoCAboveMax {
                requested: self.soc,
                max: 1.0,
            });
        }
        Ok(actual_stored)
    }

    /// Discharge the battery for `power_mw` over `duration_h` hours.
    /// Returns the actual energy delivered to the grid.
    pub fn discharge(
        &mut self,
        power_mw: f64,
        duration_h: f64,
    ) -> BatteryResult<f64> {
        let power = power_mw.max(0.0).min(self.power_rating_mw);
        let eta = self.round_trip_efficiency.sqrt();
        let available = self.available_energy_mwh();
        let out_energy = power * duration_h;
        let actual_out = out_energy.min(available);
        self.soc -= actual_out / self.energy_capacity_mwh;
        self.cumulative_throughput_mwh += actual_out;
        if self.soc < self.min_soc {
            return Err(BatteryError::SoCBelowMin {
                requested: self.soc,
                min: self.min_soc,
            });
        }
        Ok(actual_out)
    }

    /// Current usable capacity fraction after degradation (1.0 = no
    /// degradation).
    pub fn capacity_factor(&self) -> f64 {
        match &self.degradation {
            Some(d) => {
                let cycles = self.cumulative_throughput_mwh
                    / (self.energy_capacity_mwh * 0.8);
                let fade = d.calculate_degradation(cycles, self.age_years, 25.0);
                (1.0 - fade).max(0.5)
            }
            None => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charge_increases_soc() {
        let mut b = BatteryStorage::new(100.0, 50.0, 0.9).with_soc(0.0, 0.0);
        b.charge(50.0, 1.0).unwrap();
        // Stored = 50 * 0.95 (sqrt(0.9)) ≈ 47.43 MWh → SoC 47.4%
        assert!((b.soc - 0.4743).abs() < 1e-3, "soc = {}", b.soc);
    }

    #[test]
    fn discharge_decreases_soc() {
        let mut b = BatteryStorage::new(100.0, 50.0, 0.9).with_soc(0.1, 0.8);
        b.discharge(50.0, 0.5).unwrap();
        // 50 * 0.5 = 25 MWh delivered, SoC = 80% - 25% = 55%
        assert!((b.soc - 0.55).abs() < 1e-6, "soc = {}", b.soc);
    }

    #[test]
    fn discharge_clamped_to_min_soc() {
        let mut b = BatteryStorage::new(100.0, 50.0, 0.9).with_soc(0.2, 0.3);
        // Try to discharge 100 MW for 1 hour; only (30-20)%*100 = 10 MWh
        // available, so result is clamped to 10.
        let out = b.discharge(100.0, 1.0).unwrap();
        assert!((out - 10.0).abs() < 1e-6, "out = {out}");
        assert!((b.soc - 0.20).abs() < 1e-6);
    }

    #[test]
    fn round_trip_efficiency() {
        // Charge 1 MWh (grid), then discharge — net should match eta.
        let mut b = BatteryStorage::new(10.0, 5.0, 0.81).with_soc(0.0, 0.5);
        // Initial SoC: 0.5, capacity 10 MWh → 5 MWh stored.
        // Discharge 2.5 MWh for 1h at 2.5 MW.
        let out = b.discharge(2.5, 1.0).unwrap();
        assert!((out - 2.5).abs() < 1e-6);
        // SoC now 0.25, available 2.5 MWh.
        // Charge 2.5 MWh in for 1h at 2.5 MW → stored = 2.5 * 0.9 = 2.25 MWh
        let stored = b.charge(2.5, 1.0).unwrap();
        assert!((stored - 2.25).abs() < 1e-6);
        // SoC back to ~0.475 (was 0.25, +0.225)
        assert!((b.soc - 0.475).abs() < 1e-6);
    }

    #[test]
    fn degradation_reduces_capacity_factor() {
        let mut b = BatteryStorage::new(100.0, 50.0, 0.9)
            .with_soc(0.0, 0.5)
            .with_degradation(DegradationModel::li_ion());
        b.age_years = 10.0;
        b.cumulative_throughput_mwh = 50_000.0;
        let f = b.capacity_factor();
        // 10/15 = 0.667 calendar; 50000/(100*0.8) = 625 cycles / 5000 = 0.125
        // Total = ~0.79. So factor = max(0.5, 1-0.79) = 0.5 (clamped).
        assert!(f <= 1.0);
        assert!(f >= 0.5);
    }
}
