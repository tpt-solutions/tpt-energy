//! Substrate integration: use `tpt-sci-astro` to compute the Earth-Sun
//! distance correction factor for solar irradiance.
//!
//! The standard extraterrestrial irradiance `G₀ = 1361 W/m²` is at 1 AU.
//! Real GHI should be scaled by `(1 AU / r)²` where `r` is the current
//! Earth-Sun distance. `tpt-sci-astro` provides the orbital mechanics to
//! compute `r` from an Earth heliocentric orbit, which we use to validate
//! the constant `1.0` assumption used by the rest of `tpt-nrg-solar`.

/// Compute the Earth-Sun distance (AU) using `tpt-sci-astro`'s Kepler
/// propagator, given a number of days since J2000.0 TDB.
///
/// Uses the standard Earth heliocentric orbit (a = 1 AU, e ≈ 0.0167).
#[cfg(feature = "substrate")]
#[must_use]
pub fn earth_sun_distance_au(days_since_j2000: f64) -> f64 {
    use tpt_sci_astro::OrbitalElements;
    // Earth heliocentric orbit (mean anomaly advances by 2π per year).
    let a = 1.0;
    let e = 0.0167;
    let mean_motion_per_day = 2.0 * std::f64::consts::PI / 365.25;
    let m0 = 0.0_f64; // arbitrary reference epoch
    let mean_anomaly = (m0 + mean_motion_per_day * days_since_j2000).rem_euclid(2.0 * std::f64::consts::PI);
    let el = OrbitalElements::new(a, e, 0.0, 0.0, 0.0, mean_anomaly, 1.0).expect("valid earth orbit");
    let (r, _v) = el.state_vector();
    r.norm()
}

/// Correction factor `(r_0 / r)²` to scale GHI from 1 AU to the current
/// Earth-Sun distance.
#[cfg(feature = "substrate")]
#[must_use]
pub fn earth_distance_correction_ghi(days_since_j2000: f64) -> f64 {
    let r = earth_sun_distance_au(days_since_j2000);
    1.0 / (r * r)
}

#[cfg(all(test, feature = "substrate"))]
mod tests {
    use super::*;

    #[test]
    fn perihelion_is_closer_than_aphelion() {
        // Perihelion (~Jan 3) and aphelion (~Jul 4) differ by ~3.3%.
        let d_peri = earth_sun_distance_au(3.0); // ~Jan 3
        let d_aph = earth_sun_distance_au(185.0); // ~Jul 4
        assert!(d_peri < d_aph, "perihelion {d_peri} should be < aphelion {d_aph}");
        assert!((d_aph - d_peri).abs() < 0.1, "delta {d_aph} - {d_peri}");
    }

    #[test]
    fn ghi_correction_within_3pct() {
        // Eccentricity 0.0167 → variation ≈ 2 × 0.0167 ≈ 3.3%.
        for day in [0.0, 91.25, 182.5, 273.75] {
            let c = earth_distance_correction_ghi(day);
            assert!(
                (c - 1.0).abs() < 0.04,
                "day {day}: correction {c} outside ±4%"
            );
        }
    }
}
