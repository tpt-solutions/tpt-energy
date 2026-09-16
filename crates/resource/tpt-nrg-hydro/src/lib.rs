//! # tpt-nrg-hydro
//!
//! Hydroelectric power calculation: head × flow × efficiency → MW.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// Hydroelectric plant model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydroPlant {
    /// Net head in metres (after penstock losses).
    pub net_head_m: f64,
    /// Overall electromechanical efficiency (0-1), typically 0.85-0.92.
    pub efficiency: f64,
    /// Maximum turbine flow in cubic metres per second.
    pub max_flow_m3s: f64,
    /// Minimum turbine flow in cubic metres per second.
    pub min_flow_m3s: f64,
}

impl HydroPlant {
    /// Construct a new hydro plant.
    pub fn new(net_head_m: f64, efficiency: f64, max_flow_m3s: f64) -> Self {
        Self {
            net_head_m,
            efficiency,
            max_flow_m3s,
            min_flow_m3s: 0.0,
        }
    }

    /// Compute the power output (MW) for the given discharge flow.
    ///
    /// Uses the standard formula:
    /// `P [W] = ρ · g · Q · H · η` with ρ = 1000 kg/m³, g = 9.81 m/s².
    /// 1 MW = 1e6 W.
    pub fn power_output_mw(&self, flow_m3s: f64) -> f64 {
        // Below minimum turbine flow the unit is off (shut down, or
        // spilling environmentally-required flow): no generation. Clamping
        // up to `min_flow` would fabricate power out of a stopped unit.
        if flow_m3s < self.min_flow_m3s {
            return 0.0;
        }
        let q = flow_m3s.min(self.max_flow_m3s);
        1000.0 * 9.81 * q * self.net_head_m * self.efficiency / 1.0e6
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typical_plant_output() {
        // Net head 50 m, efficiency 0.90, max flow 100 m³/s
        let p = HydroPlant::new(50.0, 0.90, 100.0);
        // 1000 * 9.81 * 100 * 50 * 0.90 / 1e6 = 44.145 MW
        let out = p.power_output_mw(100.0);
        assert!((out - 44.145).abs() < 0.01, "out = {out}");
    }

    #[test]
    fn flow_clamps_to_max() {
        let p = HydroPlant::new(50.0, 0.90, 100.0);
        let out_120 = p.power_output_mw(120.0);
        let out_100 = p.power_output_mw(100.0);
        assert!((out_120 - out_100).abs() < 1e-9);
    }

    #[test]
    fn zero_flow_zero_power() {
        let p = HydroPlant::new(50.0, 0.90, 100.0);
        assert_eq!(p.power_output_mw(0.0), 0.0);
    }

    #[test]
    fn below_min_flow_unit_is_off() {
        let mut p = HydroPlant::new(50.0, 0.90, 100.0);
        p.min_flow_m3s = 5.0;
        // A shut-down unit (or one spilling env flow) must not generate.
        assert_eq!(p.power_output_mw(3.0), 0.0);
        assert_eq!(p.power_output_mw(-2.0), 0.0);
        assert!(p.power_output_mw(5.0) > 0.0);
    }
}
