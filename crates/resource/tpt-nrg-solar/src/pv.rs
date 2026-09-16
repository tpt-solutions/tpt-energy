//! PV plant configuration and DC output model.

use serde::{Deserialize, Serialize};

use crate::irradiance::{clear_sky_irradiance, plane_of_array_irradiance, Irradiance};
use crate::position::{SolarModel, SolarPosition};

/// Configuration for a PV plant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PvPlantConfig {
    /// Installed DC capacity in MWp (peak).
    pub dc_capacity_mwp: f64,
    /// DC-to-AC ratio (inverter loading ratio, e.g. 1.2 for 20% DC oversizing).
    #[serde(default = "default_dc_ac")]
    pub dc_ac_ratio: f64,
    /// Panel tilt in degrees.
    pub tilt_deg: f64,
    /// Panel azimuth in degrees (180 = south).
    pub azimuth_deg: f64,
    /// Module temperature coefficient of Pmax (per °C, typically -0.004).
    #[serde(default = "default_temp_coeff")]
    pub temp_coefficient_per_c: f64,
    /// Nominal cell operating temperature in °C (NOCT, typically 45).
    #[serde(default = "default_noct")]
    pub noct_celsius: f64,
    /// Soiling loss fraction (0-1, applied to POA irradiance).
    #[serde(default)]
    pub soiling_loss: f64,
    /// Inverter efficiency at rated load (0-1).
    #[serde(default = "default_inv_eff")]
    pub inverter_efficiency: f64,
    /// Ambient temperature in °C (used for cell-temperature estimation).
    pub ambient_celsius: f64,
}

fn default_dc_ac() -> f64 {
    1.2
}
fn default_temp_coeff() -> f64 {
    -0.004
}
fn default_noct() -> f64 {
    45.0
}
fn default_inv_eff() -> f64 {
    0.96
}

impl PvPlantConfig {
    /// Construct a new PV plant config.
    pub fn new(
        dc_capacity_mwp: f64,
        tilt_deg: f64,
        azimuth_deg: f64,
        ambient_celsius: f64,
    ) -> Self {
        Self {
            dc_capacity_mwp,
            dc_ac_ratio: default_dac(),
            tilt_deg,
            azimuth_deg,
            temp_coefficient_per_c: default_temp_coeff(),
            noct_celsius: default_noct(),
            soiling_loss: 0.02,
            inverter_efficiency: default_inv_eff(),
            ambient_celsius,
        }
    }
}

fn default_dac() -> f64 {
    1.2
}

/// A PV plant model: combines a site, config, and time-varying output
/// calculation.
#[derive(Debug, Clone)]
pub struct PvPlant {
    /// Site solar model.
    pub site: SolarModel,
    /// Plant configuration.
    pub config: PvPlantConfig,
}

impl PvPlant {
    /// Construct a new plant.
    pub fn new(site: SolarModel, config: PvPlantConfig) -> Self {
        Self { site, config }
    }

    /// Compute the AC power output at the given UTC timestamp, given a
    /// measured POA irradiance (W/m²).
    pub fn output_from_poa(&self, poa_w_per_m2: f64) -> PvOutput {
        // Soiling derating
        let poa_eff = poa_w_per_m2 * (1.0 - self.config.soiling_loss);
        // Cell temperature via NOCT model
        let cell_t =
            self.config.ambient_celsius + (self.config.noct_celsius - 20.0) * poa_eff / 800.0;
        // DC power (linear derating with temperature)
        let stc_irradiance = 1000.0;
        let p_dc = self.config.dc_capacity_mwp
            * (poa_eff / stc_irradiance)
            * (1.0 + self.config.temp_coefficient_per_c * (cell_t - 25.0));
        // Inverter clipping
        let ac_capacity_mw = self.config.dc_capacity_mwp / self.config.dc_ac_ratio;
        let p_ac_unclipped = p_dc * self.config.inverter_efficiency;
        let p_ac = p_ac_unclipped.min(ac_capacity_mw).max(0.0);
        PvOutput {
            poa_w_per_m2: poa_eff,
            cell_temperature_c: cell_t,
            dc_power_mw: p_dc.max(0.0),
            ac_power_mw: p_ac,
            inverter_clipping_mw: (p_ac_unclipped - ac_capacity_mw).max(0.0),
        }
    }

    /// Compute output using the Ineichen clear-sky model.
    pub fn output_clearsky(&self, pos: &SolarPosition) -> PvOutput {
        let linke = 3.0;
        let irrad = clear_sky_irradiance(pos, self.site.altitude_m, linke);
        self.output_from_irrad(&irrad, pos)
    }

    /// Compute output from horizontal irradiance.
    pub fn output_from_irrad(&self, irrad: &Irradiance, pos: &SolarPosition) -> PvOutput {
        let poa =
            plane_of_array_irradiance(irrad, pos, self.config.tilt_deg, self.config.azimuth_deg);
        self.output_from_poa(poa)
    }

    /// Compute output at a specific UTC timestamp using the clear-sky model.
    pub fn output_at(&self, t: chrono::DateTime<chrono::Utc>) -> PvOutput {
        let pos = self.site.solar_position(t);
        self.output_clearsky(&pos)
    }
}

/// Instantaneous PV plant output.
#[derive(Debug, Clone, Copy)]
pub struct PvOutput {
    /// Effective POA irradiance in W/m² (after soiling).
    pub poa_w_per_m2: f64,
    /// Cell temperature in °C.
    pub cell_temperature_c: f64,
    /// DC power in MW (pre-inverter).
    pub dc_power_mw: f64,
    /// AC power in MW (post-inverter, clipped).
    pub ac_power_mw: f64,
    /// Inverter clipping losses in MW.
    pub inverter_clipping_mw: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn plant() -> PvPlant {
        let site = SolarModel::new(40.0, -105.0, 1600.0, -7.0);
        let cfg = PvPlantConfig::new(100.0, 30.0, 180.0, 25.0);
        PvPlant::new(site, cfg)
    }

    fn midday_utc() -> chrono::DateTime<chrono::Utc> {
        Utc.with_ymd_and_hms(2026, 6, 21, 19, 0, 0).unwrap()
    }

    #[test]
    fn midday_output_near_capacity() {
        let p = plant();
        let out = p.output_at(midday_utc());
        // Expect 60-90% of rated AC at solar noon on a clear day.
        assert!(out.ac_power_mw > 50.0, "ac = {}", out.ac_power_mw);
    }

    #[test]
    fn night_zero_output() {
        let p = plant();
        let out = p.output_at(Utc.with_ymd_and_hms(2026, 12, 21, 11, 0, 0).unwrap());
        assert!(out.ac_power_mw.abs() < 0.1, "ac = {}", out.ac_power_mw);
    }

    #[test]
    fn cell_temperature_warmer_than_ambient() {
        let p = plant();
        let out = p.output_at(midday_utc());
        assert!(
            out.cell_temperature_c > 25.0,
            "cell_t = {}",
            out.cell_temperature_c
        );
    }

    #[test]
    fn inverter_clipping_at_overirradiance() {
        let p = plant();
        let out = p.output_from_poa(2000.0);
        // 2000 W/m² would give 200 MW DC, clipped to AC capacity = 100/1.2 ≈ 83 MW
        assert!(out.inverter_clipping_mw > 0.0);
    }
}
