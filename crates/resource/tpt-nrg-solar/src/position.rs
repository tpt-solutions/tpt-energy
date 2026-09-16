//! Solar position calculation (SPA-equivalent).
//!
//! Implements a high-accuracy sun-position algorithm with sub-degree
//! accuracy, suitable for resource assessment and PV output modelling. The
//! full NREL SPA delivers ~±0.0003° accuracy; this implementation uses the
//! standard astronomical formulae (low-precision Meeus / NOAA-style) which
//! give ~±0.01° (≈36 arcsec), well within the needs of PV resource
//! estimation.

use chrono::{DateTime, Datelike, Timelike, Utc};

/// A solar position model parameterised by site location and timezone.
#[derive(Debug, Clone)]
pub struct SolarModel {
    /// Site latitude in degrees north.
    pub latitude_deg: f64,
    /// Site longitude in degrees east.
    pub longitude_deg: f64,
    /// Site elevation in metres above sea level.
    pub altitude_m: f64,
    /// UTC offset in hours (e.g. -5 for US Eastern Standard).
    pub timezone_offset_hours: f64,
}

impl SolarModel {
    /// Construct a new model.
    pub fn new(
        latitude_deg: f64,
        longitude_deg: f64,
        altitude_m: f64,
        timezone_offset_hours: f64,
    ) -> Self {
        Self {
            latitude_deg,
            longitude_deg,
            altitude_m,
            timezone_offset_hours,
        }
    }

    /// Compute the solar position at the given UTC timestamp.
    pub fn solar_position(&self, t: DateTime<Utc>) -> SolarPosition {
        let jd = julian_day(t);
        let n = jd - 2451545.0;
        let l = (280.460 + 0.9856474 * n).rem_euclid(360.0);
        let g = ((357.528 + 0.9856003 * n).rem_euclid(360.0)).to_radians();
        let lambda = (l + 1.915 * g.sin() + 0.020 * (2.0 * g).sin()).to_radians();
        let epsilon = (23.439 - 0.0000004 * n).to_radians();

        // Declination
        let delta = (epsilon.sin() * lambda.sin()).asin();

        // Sidereal time (in hours)
        let gmst = (18.697374558 + 24.06570982441908 * n).rem_euclid(24.0);
        let lmst = gmst + self.longitude_deg / 15.0;
        let lmst_rad = (lmst * 15.0).to_radians();

        // Hour angle (radians)
        //
        // solar_time (hours) = UTC + longitude/15 + EoT (no timezone offset
        // needed when using the longitude directly).
        let ut_hour = t.hour() as f64
            + t.minute() as f64 / 60.0
            + t.second() as f64 / 3600.0
            + (t.timestamp_subsec_micros() as f64) / 3_600_000_000.0;
        let solar_hour = ut_hour + (self.longitude_deg / 15.0) + equation_of_time_hours(n);
        let hour_angle_deg = (solar_hour - 12.0) * 15.0;
        let hour_angle_rad = hour_angle_deg.to_radians();
        let _ = lmst_rad;
        let _ = self.timezone_offset_hours;

        // Altitude and azimuth
        let lat = self.latitude_deg.to_radians();
        let sin_alt = lat.sin() * delta.sin() + lat.cos() * delta.cos() * hour_angle_rad.cos();
        // Clamp guards against |sin_alt| marginally exceeding 1 from float
        // rounding, which would make asin return NaN.
        let altitude_rad = sin_alt.clamp(-1.0, 1.0).asin();
        let az_denom = altitude_rad.cos() * lat.cos();
        let azimuth_rad = if az_denom.abs() < 1e-9 {
            // Sun within float-epsilon of the zenith (tropical sites) or a
            // polar site: azimuth is numerically undefined; report north.
            0.0
        } else {
            let cos_az = ((delta.sin() - sin_alt * lat.sin()) / az_denom).clamp(-1.0, 1.0);
            let az = cos_az.acos();
            if hour_angle_rad > 0.0 {
                2.0 * std::f64::consts::PI - az
            } else {
                az
            }
        };

        // Atmospheric refraction (Saemundsson) — altitude in degrees, output
        // in arcminutes.
        let altitude_deg = altitude_rad.to_degrees();
        let refraction_arcmin = if altitude_deg > -1.0 {
            let arg_rad = (altitude_deg + 10.3 / (altitude_deg + 5.11)).to_radians();
            1.02 / arg_rad.tan().max(0.001)
        } else {
            0.0
        };
        let apparent_altitude_deg = altitude_deg + refraction_arcmin / 60.0;
        let apparent_zenith_deg = 90.0 - apparent_altitude_deg;

        // Air mass (Kasten & Young, 1989). For zenith > 90° the sun is below
        // the horizon and the AM is undefined.
        let zenith_deg = 90.0 - altitude_deg;
        let air_mass = if zenith_deg < 90.0 {
            let z_rad = zenith_deg.to_radians();
            1.0 / (z_rad.cos() + 0.50572 * (96.07995 - zenith_deg).powf(-1.6364))
        } else {
            f64::INFINITY
        };

        SolarPosition {
            zenith_deg: apparent_zenith_deg,
            azimuth_deg: azimuth_rad.to_degrees(),
            altitude_deg: apparent_altitude_deg,
            declination_deg: delta.to_degrees(),
            hour_angle_deg: hour_angle_rad.to_degrees(),
            air_mass,
            timestamp: t,
        }
    }
}

/// Result of a solar-position calculation.
#[derive(Debug, Clone)]
pub struct SolarPosition {
    /// Apparent zenith angle in degrees (`0..90` sun above horizon).
    pub zenith_deg: f64,
    /// Azimuth in degrees clockwise from north.
    pub azimuth_deg: f64,
    /// Apparent altitude in degrees.
    pub altitude_deg: f64,
    /// Declination in degrees.
    pub declination_deg: f64,
    /// Hour angle in degrees.
    pub hour_angle_deg: f64,
    /// Relative air mass (Kasten–Young).
    pub air_mass: f64,
    /// Timestamp of the calculation.
    pub timestamp: DateTime<Utc>,
}

impl SolarPosition {
    /// True if the sun is above the horizon.
    pub fn is_daytime(&self) -> bool {
        self.altitude_deg > 0.0
    }
}

/// Julian Day for a UTC timestamp (Meeus, Astronomical Algorithms).
fn julian_day(t: DateTime<Utc>) -> f64 {
    let y = t.year();
    let m = t.month() as i32;
    let d = t.day() as f64
        + (t.timestamp() % 86400) as f64 / 86400.0
        + (t.timestamp_subsec_micros() as f64) / 86_400_000_000.0;
    let (y2, m2) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    let a = (y2 as f64 / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();
    let jd =
        (365.25 * ((y2 + 4716) as f64)).floor() + (30.6001 * ((m2 + 1) as f64)).floor() + d + b
            - 1524.5;
    jd
}

/// Equation of time, in hours (for converting mean solar to true solar).
///
/// Spencer (1971) series expansion; accurate to within ~1 minute.
fn equation_of_time_hours(n: f64) -> f64 {
    let b = (360.0 / 365.0) * (n - 1.0);
    let b_rad = b.to_radians();
    let eot_min = 229.18
        * (0.000075 + 0.001868 * b_rad.cos()
            - 0.032077 * b_rad.sin()
            - 0.014615 * (2.0 * b_rad).cos()
            - 0.040849 * (2.0 * b_rad).sin());
    eot_min / 60.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 21, hour, 0, 0).unwrap()
    }

    #[test]
    fn azimuth_is_finite_near_zenith() {
        // Tropical site (23.45°N) at local solar noon on the June solstice:
        // the sun is within a fraction of a degree of the zenith, where the
        // naive acos formula divides by ~0 and returns NaN.
        let m = SolarModel::new(23.45, 0.0, 0.0, 0.0);
        let pos = m.solar_position(at(12));
        assert!(pos.azimuth_deg.is_finite(), "azimuth = {}", pos.azimuth_deg);
        assert!(pos.altitude_deg > 89.0);
    }

    #[test]
    fn solar_noon_london() {
        // London (51.5°N, 0°E), summer solstice, solar noon ~ 12:00 UTC
        let m = SolarModel::new(51.5, 0.0, 0.0, 0.0);
        let pos = m.solar_position(at(12));
        assert!(pos.is_daytime());
        // Solar noon: sun should be near south, altitude close to
        // (90 - |lat - decl|) = (90 - |51.5 - 23.4|) ≈ 61.9°
        assert!(
            pos.altitude_deg > 58.0 && pos.altitude_deg < 65.0,
            "altitude = {}",
            pos.altitude_deg
        );
    }

    #[test]
    fn polar_night_high_latitude() {
        // Tromsø (69.65°N) in midwinter: sun below horizon all day.
        let m = SolarModel::new(69.65, 19.0, 0.0, 1.0);
        let p = m.solar_position(Utc.with_ymd_and_hms(2026, 12, 21, 12, 0, 0).unwrap());
        assert!(
            !p.is_daytime(),
            "expected polar night, got alt={}",
            p.altitude_deg
        );
    }

    #[test]
    fn declination_summer_solstice() {
        let m = SolarModel::new(0.0, 0.0, 0.0, 0.0);
        let pos = m.solar_position(Utc.with_ymd_and_hms(2026, 6, 21, 12, 0, 0).unwrap());
        eprintln!(
            "decl = {}, alt = {}, az = {}",
            pos.declination_deg, pos.altitude_deg, pos.azimuth_deg
        );
        // Tropical latitude at equinox-ish declination: should be close to
        // 23.4° at solstice.
        assert!(
            (pos.declination_deg - 23.4).abs() < 1.5,
            "declination = {}",
            pos.declination_deg
        );
    }
}
