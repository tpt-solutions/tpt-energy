//! Clear-sky irradiance models (Ineichen) and plane-of-array transposition.

use serde::{Deserialize, Serialize};

use chrono::Datelike;

use crate::position::SolarPosition;

/// Sky condition flag for clear-sky modelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkyCondition {
    /// Standard clear sky (Ineichen default).
    Clear,
    /// Slightly hazy.
    Hazy,
    /// Partly cloudy.
    PartlyCloudy,
}

/// Clear-sky irradiance components at a horizontal surface.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Irradiance {
    /// Global horizontal irradiance in W/m².
    pub ghi_w_per_m2: f64,
    /// Direct normal irradiance in W/m².
    pub dni_w_per_m2: f64,
    /// Diffuse horizontal irradiance in W/m².
    pub dhi_w_per_m2: f64,
}

impl Irradiance {
    /// Zero irradiance (night).
    pub const ZERO: Self = Self {
        ghi_w_per_m2: 0.0,
        dni_w_per_m2: 0.0,
        dhi_w_per_m2: 0.0,
    };
}

/// Compute clear-sky irradiance using the Ineichen model.
///
/// `altitude_m` is the site elevation in metres, `linke_turbidity` the
/// Linke atmospheric turbidity (typical 2-6).
pub fn clear_sky_irradiance(
    pos: &SolarPosition,
    altitude_m: f64,
    linke_turbidity: f64,
) -> Irradiance {
    if !pos.is_daytime() {
        return Irradiance::ZERO;
    }
    let zenith_deg = pos.zenith_deg;
    let zenith_rad = zenith_deg.to_radians();
    let cos_zenith = zenith_rad.cos();
    if cos_zenith <= 0.0 {
        return Irradiance::ZERO;
    }
    // Extraterrestrial normal irradiance (solar constant with seasonal
    // variation due to Earth-Sun distance).
    let day_of_year = pos.timestamp.ordinal() as f64;
    let e0 = 1361.0 * (1.0 + 0.033 * (2.0 * std::f64::consts::PI * day_of_year / 365.0).cos());
    let am = pos.air_mass;
    if !am.is_finite() || am <= 0.0 {
        return Irradiance::ZERO;
    }
    // Ineichen clear-sky model. Higher elevation means a shorter, thinner
    // atmospheric path, so the optical-depth exponent is scaled down with a
    // barometric pressure ratio (scale height ~8000 m).
    let a = 1.154;
    let b = -0.154;
    let c = (0.98 - 0.00146 * linke_turbidity).max(0.3);
    let pressure_ratio = (-altitude_m / 8000.0).exp();
    let dni = a * e0 * (b * am * pressure_ratio).exp();
    let dni_capped = dni.max(0.0).min(1400.0);

    // Diffuse horizontal from Ineichen (simplified)
    let dhi = 0.05
        * e0
        * (90.0 - zenith_deg).to_radians().sin().max(0.0)
        * (-1.0 * (linke_turbidity - 1.0) / 8.0).exp();
    // `c` is the turbidity-derived atmospheric clearness factor applied to
    // total global irradiance.
    let ghi = (dni_capped * cos_zenith + dhi) * c;

    Irradiance {
        ghi_w_per_m2: ghi.max(0.0),
        dni_w_per_m2: dni_capped,
        dhi_w_per_m2: dhi.max(0.0),
    }
}

/// Compute the plane-of-array (POA) irradiance for a tilted surface.
///
/// Uses the isotropic-sky diffuse model (Liu & Jordan).
///
/// `tilt_deg` is the panel tilt from horizontal (0 = flat).
/// `azimuth_deg` is the panel azimuth (180 = south in northern hemisphere).
pub fn plane_of_array_irradiance(
    horiz: &Irradiance,
    pos: &SolarPosition,
    tilt_deg: f64,
    azimuth_deg: f64,
) -> f64 {
    if !pos.is_daytime() {
        return 0.0;
    }
    let tilt = tilt_deg.to_radians();
    let az = azimuth_deg.to_radians();
    let sun_az = pos.azimuth_deg.to_radians();
    let sun_zen = pos.zenith_deg.to_radians();
    // Angle of incidence on tilted surface
    let cos_aoi = (sun_zen.sin() * tilt.sin() * (sun_az - az).cos()) + (sun_zen.cos() * tilt.cos());
    let beam = if cos_aoi > 0.0 {
        horiz.dni_w_per_m2 * cos_aoi
    } else {
        0.0
    };
    let diffuse = horiz.dhi_w_per_m2 * (1.0 + tilt.cos()) / 2.0;
    let ground = horiz.ghi_w_per_m2 * 0.2 * (1.0 - tilt.cos()) / 2.0;
    (beam + diffuse + ground).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::SolarModel;
    use chrono::{TimeZone, Utc};

    fn midday() -> SolarPosition {
        let m = SolarModel::new(40.0, -105.0, 1600.0, -7.0);
        // Solar noon at lon -105 with tz=-7 (MDT): ~19:00 UTC.
        m.solar_position(Utc.with_ymd_and_hms(2026, 6, 21, 19, 0, 0).unwrap())
    }

    #[test]
    fn clear_sky_dni_positive_at_midday() {
        let pos = midday();
        let irrad = clear_sky_irradiance(&pos, 1600.0, 2.5);
        eprintln!(
            "alt={}, az={}, dni={}, ghi={}, dhi={}, am={}",
            pos.altitude_deg,
            pos.azimuth_deg,
            irrad.dni_w_per_m2,
            irrad.ghi_w_per_m2,
            irrad.dhi_w_per_m2,
            pos.air_mass
        );
        // Expected: ~900-1050 W/m² for DNI at solar noon in summer.
        assert!(irrad.dni_w_per_m2 > 600.0, "dni = {}", irrad.dni_w_per_m2);
        assert!(irrad.ghi_w_per_m2 > 400.0, "ghi = {}", irrad.ghi_w_per_m2);
    }

    #[test]
    fn poa_increases_on_tilted_surface() {
        let pos = midday();
        let irrad = clear_sky_irradiance(&pos, 1600.0, 2.5);
        let flat = plane_of_array_irradiance(&irrad, &pos, 0.0, 180.0);
        let tilted = plane_of_array_irradiance(&irrad, &pos, 30.0, 180.0);
        assert!(tilted > flat, "tilted {tilted} should exceed flat {flat}");
    }

    #[test]
    fn night_returns_zero() {
        let m = SolarModel::new(40.0, -105.0, 1600.0, -7.0);
        let pos = m.solar_position(Utc.with_ymd_and_hms(2026, 12, 21, 4, 0, 0).unwrap());
        let irrad = clear_sky_irradiance(&pos, 1600.0, 2.5);
        // Allow tiny numerical noise but the night-time irradiance must be ~0.
        assert!(irrad.ghi_w_per_m2 < 1.0, "ghi = {}", irrad.ghi_w_per_m2);
        assert!(irrad.dni_w_per_m2 < 1.0, "dni = {}", irrad.dni_w_per_m2);
    }
}
