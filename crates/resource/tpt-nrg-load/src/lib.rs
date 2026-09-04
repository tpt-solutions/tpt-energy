//! # tpt-nrg-load
//!
//! Load forecasting and demand-response models.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// A load model: base load, temperature sensitivity, and price elasticity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadModel {
    /// Baseline load in MW.
    pub base_load_mw: f64,
    /// Temperature sensitivity in MW / °C (positive = heating-dominated,
    /// negative = cooling-dominated).
    pub temperature_sensitivity_mw_per_c: f64,
    /// Reference temperature in °C at which the base load applies.
    pub reference_temperature_c: f64,
    /// Price elasticity of demand: `ΔL/ΔP` in MW per $/MWh (typically
    /// negative — higher prices reduce demand).
    pub price_elasticity_mw_per_dollar_per_mwh: f64,
    /// Diurnal / weekly multipliers (24 hours × 7 days). If empty, treated
    /// as all 1.0.
    #[serde(default)]
    pub shape: Vec<f64>,
}

impl LoadModel {
    /// Construct a simple load model with constant shape.
    pub fn new(base_load_mw: f64) -> Self {
        Self {
            base_load_mw,
            temperature_sensitivity_mw_per_c: 0.0,
            reference_temperature_c: 20.0,
            price_elasticity_mw_per_dollar_per_mwh: 0.0,
            shape: Vec::new(),
        }
    }

    /// Set temperature sensitivity.
    pub fn with_temperature_sensitivity(mut self, mw_per_c: f64, ref_c: f64) -> Self {
        self.temperature_sensitivity_mw_per_c = mw_per_c;
        self.reference_temperature_c = ref_c;
        self
    }

    /// Set the price elasticity.
    pub fn with_price_elasticity(mut self, mw_per_dollar_per_mwh: f64) -> Self {
        self.price_elasticity_mw_per_dollar_per_mwh = mw_per_dollar_per_mwh;
        self
    }

    /// Set the diurnal shape: 168 multipliers (24h × 7d). Values must be
    /// non-negative.
    pub fn with_weekly_shape(mut self, shape: Vec<f64>) -> Self {
        self.shape = shape;
        self
    }

    /// Forecast the load (MW) for the given hour-of-week (0..168), ambient
    /// temperature, and electricity price.
    pub fn forecast_load(
        &self,
        hour_of_week: usize,
        temperature_c: f64,
        price_dollar_per_mwh: f64,
    ) -> f64 {
        let shape_mult = if self.shape.is_empty() {
            1.0
        } else {
            self.shape[hour_of_week.min(self.shape.len() - 1)]
        };
        let temp_term = self
            .temperature_sensitivity_mw_per_c
            * (temperature_c - self.reference_temperature_c);
        let price_term = self.price_elasticity_mw_per_dollar_per_mwh * price_dollar_per_mwh;
        ((self.base_load_mw + temp_term + price_term) * shape_mult).max(0.0)
    }

    /// Compute the demand-response load reduction (MW) for a given price
    /// signal relative to a reference price.
    pub fn demand_response(&self, price_dollar_per_mwh: f64, reference_price: f64) -> f64 {
        let delta = price_dollar_per_mwh - reference_price;
        (-self.price_elasticity_mw_per_dollar_per_mwh * delta).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_load_no_shape() {
        let m = LoadModel::new(100.0);
        assert!((m.forecast_load(0, 20.0, 50.0) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn temperature_increases_load() {
        let m = LoadModel::new(100.0).with_temperature_sensitivity(2.0, 20.0);
        // At 25°C with positive sensitivity (heating-dominated in winter
        // would decrease, but positive here means cold → higher load), we
        // get load = 100 + 2*5 = 110 MW.
        assert!((m.forecast_load(0, 25.0, 50.0) - 110.0).abs() < 1e-9);
        // At 15°C, load = 100 + 2*(-5) = 90 MW.
        assert!((m.forecast_load(0, 15.0, 50.0) - 90.0).abs() < 1e-9);
    }

    #[test]
    fn price_elasticity_reduces_load() {
        let m = LoadModel::new(100.0).with_price_elasticity(-0.5);
        // Elasticity = -0.5 MW/($/MWh): at price $100 (ΔP=50), reduction = 25 MW
        let dr = m.demand_response(100.0, 50.0);
        assert!((dr - 25.0).abs() < 1e-9, "dr = {dr}");
    }

    #[test]
    fn weekly_shape_modulates() {
        let mut shape = vec![1.0; 168];
        shape[18] = 1.5; // Monday 6pm peak
        let m = LoadModel::new(100.0).with_weekly_shape(shape);
        assert!((m.forecast_load(18, 20.0, 50.0) - 150.0).abs() < 1e-9);
    }
}
