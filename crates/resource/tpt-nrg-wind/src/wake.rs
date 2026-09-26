//! Wind farm wake-loss models.
//!
//! Three analytical models are provided, all sharing the same geometry
//! convention: each downstream turbine is waked by every upstream turbine
//! whose wake cone overlaps it, and deficits multiply (kinematic
//! superposition of `v_eff / v_free`).
//!
//! - [`WakeModel::JensenPark`] — top-hat wake, linear expansion, full
//!   deficit inside the wake cone.
//! - [`WakeModel::Frandsen`] — rotor-equivalent wake source with a
//!   two-zone (near/far) axial deficit and partial-rotor overlap
//!   weighting, which matters for closely spaced turbines.
//! - [`WakeModel::EddyViscosity`] — a simple explicit eddy-viscosity
//!   march that diffuses the initial top-hat deficit downstream,
//!   producing a smooth, radially spreading wake.

use serde::{Deserialize, Serialize};

use crate::turbine::WindTurbine;

/// Wake model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WakeModel {
    /// Jensen / PARK model — top-hat wake with linear expansion.
    JensenPark,
    /// Frandsen-type model — rotor-equivalent source, two-zone deficit,
    /// partial-rotor overlap.
    Frandsen,
    /// Simple eddy-viscosity model — diffusive downstream march.
    EddyViscosity,
}

/// A wind farm layout: turbines positioned in (x, y) metres relative to a
/// reference point, with a common prevailing wind direction (degrees from
/// north, blowing toward).
#[derive(Debug, Clone)]
pub struct WindFarm {
    /// Turbine positions: `[(x_m, y_m, turbine)]`.
    pub turbines: Vec<(f64, f64, WindTurbine)>,
    /// Wake model to use.
    pub wake_model: WakeModel,
    /// Wake decay coefficient `k` (typically 0.05 for onshore, 0.04 for
    /// offshore). Used by all three models.
    pub wake_decay_k: f64,
    /// Prevailing wind direction in degrees (the direction the wind is
    /// blowing toward). Each downstream turbine experiences a wake from
    /// every upstream turbine in the same wind line.
    pub wind_direction_deg: f64,
}

/// Number of sample points used to average a wake profile across a
/// downstream rotor disk.
const ROTOR_SAMPLES: usize = 8;

/// Convert a loop index or sample count to `f64` for arithmetic.
///
/// Exact for magnitudes up to 2^52, far beyond any realistic loop bound;
/// the precision-loss allowance lives at this single documented point.
#[must_use]
#[allow(clippy::cast_precision_loss)]
fn count_to_f64(value: usize) -> f64 {
    value as f64
}

/// Grid-point count from a `ceil`'d ratio.
///
/// Ratios are bounded by realistic grid extents (hundreds of points), so
/// the conversion cannot truncate or go negative.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn grid_points(value: f64) -> usize {
    value as usize
}

impl WindFarm {
    /// Construct a wind farm.
    #[must_use]
    pub fn new(wake_model: WakeModel, wind_direction_deg: f64) -> Self {
        Self {
            turbines: Vec::new(),
            wake_model,
            wake_decay_k: 0.05,
            wind_direction_deg,
        }
    }

    /// Add a turbine at the given position.
    pub fn push(&mut self, x_m: f64, y_m: f64, turbine: WindTurbine) {
        self.turbines.push((x_m, y_m, turbine));
    }

    /// Set the wake decay coefficient.
    #[must_use]
    pub fn with_wake_decay(mut self, k: f64) -> Self {
        self.wake_decay_k = k;
        self
    }

    /// Compute the effective wind speed at each turbine, applying wake
    /// losses from all upstream turbines under the farm's [`WakeModel`].
    #[must_use]
    pub fn effective_wind_speeds(&self, free_stream_mps: f64) -> Vec<f64> {
        let n = self.turbines.len();
        let mut v_eff = vec![free_stream_mps; n];
        if n == 0 {
            return v_eff;
        }
        let (dir_sin, dir_cos) = self.wind_direction_rad().sin_cos();
        for (i, &(xi, yi, _)) in self.turbines.iter().enumerate() {
            let mut factor = 1.0_f64;
            for (j, (xj, yj, upstream)) in self.turbines.iter().enumerate() {
                if i == j {
                    continue;
                }
                // Vector from the upstream turbine j to the target i.
                let dx = xi - xj;
                let dy = yi - yj;
                // Downstream if (i - j) projected on the wind direction is
                // positive.
                let down = dx * dir_sin + dy * dir_cos;
                if down <= 0.0 {
                    continue;
                }
                let lateral = (dx * dir_cos - dy * dir_sin).abs();
                factor *= self.wake_multiplier(upstream, down, lateral);
            }
            v_eff[i] = free_stream_mps * factor;
        }
        v_eff
    }

    /// Total farm power output (MW) at the given free-stream wind speed.
    #[must_use]
    pub fn total_power_output(&self, free_stream_mps: f64) -> f64 {
        let v_eff = self.effective_wind_speeds(free_stream_mps);
        let mut total = 0.0;
        for (i, (_, _, t)) in self.turbines.iter().enumerate() {
            total += t.power_at(v_eff[i]);
        }
        total
    }

    /// Multiplicative wind-speed factor at a point `down` metres downstream
    /// and `lateral` metres off the wake centerline of `upstream`.
    fn wake_multiplier(&self, upstream: &WindTurbine, down: f64, lateral: f64) -> f64 {
        match self.wake_model {
            WakeModel::JensenPark => self.jensen_multiplier(upstream, down, lateral),
            WakeModel::Frandsen => self.frandsen_multiplier(upstream, down, lateral),
            WakeModel::EddyViscosity => self.eddy_viscosity_multiplier(upstream, down, lateral),
        }
    }

    /// Jensen / PARK: `deficit = (1 - sqrt(1 - C_T)) * (D / (D + 2kx))^2`,
    /// applied in full inside the wake cone `|lateral| < D/2 + kx` and not
    /// at all outside it.
    fn jensen_multiplier(&self, upstream: &WindTurbine, down: f64, lateral: f64) -> f64 {
        let d = upstream.rotor_diameter_m;
        let ct = upstream.thrust_coefficient;
        let dw = d + 2.0 * self.wake_decay_k * down;
        let deficit = (1.0 - (1.0 - ct).sqrt()) * (d / dw).powi(2);
        let wake_radius = d * 0.5 + self.wake_decay_k * down;
        if lateral < wake_radius {
            1.0 - deficit
        } else {
            1.0
        }
    }

    /// Frandsen-type: the wake starts from a rotor-equivalent source of
    /// radius `D/2 * sqrt((1 + sqrt(1 - C_T)) / 2)`, keeps the initial
    /// deficit `1 - sqrt(1 - C_T)` through the near wake, then follows the
    /// momentum-conserving far-wake law
    /// `deficit = (1 - sqrt(1 - C_T (D / D_w)^2)) / 2`. The deficit is
    /// weighted by the fraction of the downstream rotor area overlapped by
    /// the wake disk, which is what makes the model better behaved for
    /// closely spaced (partially waked) turbines.
    fn frandsen_multiplier(&self, upstream: &WindTurbine, down: f64, lateral: f64) -> f64 {
        let d = upstream.rotor_diameter_m;
        let ct = upstream.thrust_coefficient.clamp(1.0e-3, 1.0 - 1.0e-12);
        let k = self.wake_decay_k;
        let source_radius = d * 0.5 * ((1.0 + (1.0 - ct).sqrt()) / 2.0).sqrt();
        let wake_radius = source_radius + k * down;
        // Near-wake length: the downstream distance at which the wake
        // diameter has expanded to `D * sqrt(C_T)`, where the far-wake
        // argument first becomes valid.
        let near_wake_length = (d * ct.sqrt() * 0.5 - source_radius) / k;
        let deficit = if down <= near_wake_length {
            0.5 * (1.0 - (1.0 - ct).sqrt())
        } else {
            let dw = 2.0 * wake_radius;
            0.5 * (1.0 - (1.0 - ct * (d / dw).powi(2)).max(0.0).sqrt())
        };
        let rotor_radius = d * 0.5;
        let overlap = disk_overlap_fraction(rotor_radius, wake_radius, lateral);
        1.0 - deficit * overlap
    }

    /// Simple eddy-viscosity: march the initial top-hat deficit
    /// (`1 - sqrt(1 - C_T)` across the rotor) downstream with a cylindrical
    /// diffusion step driven by an eddy viscosity that grows linearly with
    /// downstream distance (`nu_hat = k * x_hat`), then average the
    /// resulting deficit profile across the downstream rotor disk.
    fn eddy_viscosity_multiplier(&self, upstream: &WindTurbine, down: f64, lateral: f64) -> f64 {
        let d = upstream.rotor_diameter_m;
        let ct = upstream.thrust_coefficient.clamp(1.0e-3, 1.0);
        let rotor_radius = d * 0.5;
        let initial_deficit = 1.0 - (1.0 - ct).sqrt();
        let profile =
            EddyViscosityProfile::new(rotor_radius, initial_deficit, self.wake_decay_k, down);
        // Average the deficit over the rotor cross-section (symmetric
        // profile, so a signed offset and its absolute value agree).
        let mut sum = 0.0_f64;
        for s in 0..ROTOR_SAMPLES {
            let frac = count_to_f64(s) / count_to_f64(ROTOR_SAMPLES - 1);
            let r = lateral - rotor_radius + 2.0 * rotor_radius * frac;
            sum += profile.deficit_at(r.abs());
        }
        1.0 - sum / count_to_f64(ROTOR_SAMPLES)
    }

    fn wind_direction_rad(&self) -> f64 {
        // Convention: "direction toward which the wind is blowing" in
        // degrees clockwise from north.
        self.wind_direction_deg.to_radians()
    }
}

/// Area fraction of the rotor disk overlapped by the wake disk, given the
/// lateral offset between their centers.
fn disk_overlap_fraction(rotor_radius: f64, wake_radius: f64, center_offset: f64) -> f64 {
    let rotor_area = std::f64::consts::PI * rotor_radius * rotor_radius;
    if rotor_area <= 0.0 || center_offset >= rotor_radius + wake_radius {
        return 0.0;
    }
    if center_offset <= (wake_radius - rotor_radius).abs() {
        // One disk fully contains the other.
        return if wake_radius >= rotor_radius {
            1.0
        } else {
            wake_radius * wake_radius / (rotor_radius * rotor_radius)
        };
    }
    // Standard circle-circle lens area.
    let d = center_offset;
    let alpha = ((d * d + rotor_radius * rotor_radius - wake_radius * wake_radius)
        / (2.0 * d * rotor_radius))
        .clamp(-1.0, 1.0)
        .acos();
    let beta = ((d * d + wake_radius * wake_radius - rotor_radius * rotor_radius)
        / (2.0 * d * wake_radius))
        .clamp(-1.0, 1.0)
        .acos();
    let lens = rotor_radius * rotor_radius * (alpha - alpha.sin() * alpha.cos())
        + wake_radius * wake_radius * (beta - beta.sin() * beta.cos());
    (lens / rotor_area).clamp(0.0, 1.0)
}

/// Diffused wake-deficit profile for the simple eddy-viscosity model.
///
/// The profile lives on a radial grid normalized by the rotor radius
/// (`r_hat = r / R`, `dr_hat = 0.25`) and is advanced from the source to
/// the requested downstream distance `x_hat = x / R` with an explicit
/// cylindrical diffusion step. The eddy viscosity grows linearly with
/// downstream distance, `nu_hat(x_hat) = k * x_hat`, which is the classic
/// thin-shear-layer closure scaled by the wake decay constant.
struct EddyViscosityProfile {
    /// Radial grid in rotor radii from the wake centerline.
    r_hat: Vec<f64>,
    /// Velocity deficit at each grid point.
    deficit: Vec<f64>,
    /// Rotor radius in metres (to convert queries).
    rotor_radius: f64,
}

impl EddyViscosityProfile {
    fn new(rotor_radius: f64, initial_deficit: f64, wake_decay_k: f64, down: f64) -> Self {
        let dr_hat = 0.25_f64;
        // Extend the grid past the widest the wake can plausibly get.
        let r_hat_max = 3.0 + wake_decay_k * (down / rotor_radius) * 1.5;
        let n_radial = usize::max(4, grid_points((r_hat_max / dr_hat).ceil()) + 1);
        let mut r_hat = Vec::with_capacity(n_radial);
        let mut deficit = Vec::with_capacity(n_radial);
        for i in 0..n_radial {
            let r = dr_hat * count_to_f64(i);
            r_hat.push(r);
            deficit.push(if r <= 1.0 { initial_deficit } else { 0.0 });
        }

        // Stability of the explicit step requires x_step <= dr_hat^2/(2
        // nu_max) for the interior nodes; the node adjacent to the axis is
        // stiffer by a factor ~2, so use a quarter of the interior limit
        // and cap the step so the source region is resolved too.
        let x_hat_end = (down / rotor_radius).max(f64::MIN_POSITIVE);
        let nu_max = wake_decay_k * x_hat_end;
        let x_step = (dr_hat * dr_hat / (4.0 * nu_max)).min(0.02);
        let n_steps = usize::max(1, grid_points((x_hat_end / x_step).ceil()));
        let x_step = x_hat_end / count_to_f64(n_steps);

        for step in 0..n_steps {
            let x_hat = (count_to_f64(step) + 0.5) * x_step;
            let nu = wake_decay_k * x_hat;
            let mut next = deficit.clone();
            // Axis (r_hat = 0): cylindrical Laplacian reduces to 4 d/dr.
            next[0] = (deficit[0]
                + nu * x_step * 4.0 * (deficit[1] - deficit[0]) / (dr_hat * dr_hat))
                .max(0.0);
            // Interior: d(deficit)/x_step = nu_hat / r_hat * d/dr_hat(r_hat
            // d(deficit)/dr_hat).
            for i in 1..n_radial - 1 {
                let r_lo = r_hat[i] - 0.5 * dr_hat;
                let r_hi = r_hat[i] + 0.5 * dr_hat;
                let flux =
                    r_hi * (deficit[i + 1] - deficit[i]) - r_lo * (deficit[i] - deficit[i - 1]);
                next[i] = (deficit[i] + nu * x_step * flux / (r_hat[i] * dr_hat * dr_hat)).max(0.0);
            }
            // Outer boundary: free stream.
            next[n_radial - 1] = 0.0;
            deficit = next;
        }

        Self {
            r_hat,
            deficit,
            rotor_radius,
        }
    }

    /// Linear interpolation of the deficit at radius `r` (metres).
    fn deficit_at(&self, r: f64) -> f64 {
        let r_hat = r / self.rotor_radius;
        if r_hat <= 0.0 {
            return self.deficit[0];
        }
        let last = self.r_hat.len() - 1;
        if r_hat >= self.r_hat[last] {
            return 0.0;
        }
        let dr_hat = self.r_hat[1] - self.r_hat[0];
        let idx = r_hat / dr_hat;
        let lower = grid_points(idx.floor());
        let frac = idx - count_to_f64(lower);
        self.deficit[lower] * (1.0 - frac) + self.deficit[lower + 1] * frac
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::turbine::WindTurbine;

    fn turbine() -> WindTurbine {
        WindTurbine::new("T", 80.0, 2.0, 3.0, 12.0, 25.0)
    }

    #[test]
    fn no_wake_for_single_turbine() {
        for model in [
            WakeModel::JensenPark,
            WakeModel::Frandsen,
            WakeModel::EddyViscosity,
        ] {
            let mut farm = WindFarm::new(model, 90.0);
            farm.push(0.0, 0.0, turbine());
            let v = farm.effective_wind_speeds(10.0);
            assert!((v[0] - 10.0).abs() < 1e-9, "{model:?}");
        }
    }

    #[test]
    fn downstream_turbine_slower() {
        for model in [
            WakeModel::JensenPark,
            WakeModel::Frandsen,
            WakeModel::EddyViscosity,
        ] {
            let mut farm = WindFarm::new(model, 90.0).with_wake_decay(0.05);
            // Two turbines 5 diameters apart, wind blowing east (90°)
            farm.push(0.0, 0.0, turbine());
            farm.push(400.0, 0.0, turbine()); // 5D downstream
            let v = farm.effective_wind_speeds(10.0);
            assert!((v[0] - 10.0).abs() < 1e-6, "{model:?}");
            assert!(v[1] < 10.0, "{model:?}");
            assert!(v[1] > 5.0, "{model:?}: v[1] = {}", v[1]);
        }
    }

    #[test]
    fn lateral_turbine_unaffected() {
        for model in [
            WakeModel::JensenPark,
            WakeModel::Frandsen,
            WakeModel::EddyViscosity,
        ] {
            let mut farm = WindFarm::new(model, 90.0).with_wake_decay(0.05);
            // Two turbines well outside the wake cone
            farm.push(0.0, 0.0, turbine());
            farm.push(400.0, 500.0, turbine()); // 5D downstream, 6.25D lateral
            let v = farm.effective_wind_speeds(10.0);
            assert!((v[1] - 10.0).abs() < 1e-6, "{model:?}: v[1] = {}", v[1]);
        }
    }

    #[test]
    fn total_power_loss_in_farm() {
        for model in [
            WakeModel::JensenPark,
            WakeModel::Frandsen,
            WakeModel::EddyViscosity,
        ] {
            let mut farm = WindFarm::new(model, 90.0).with_wake_decay(0.05);
            // 4 turbines in a row, 5D apart
            for i in 0..4 {
                farm.push(f64::from(i) * 400.0, 0.0, turbine());
            }
            let p_farm = farm.total_power_output(12.0);
            let p_single = turbine().power_at(12.0);
            assert!(
                p_farm < 4.0 * p_single,
                "{model:?}: p_farm={p_farm}, 4*p={}",
                4.0 * p_single
            );
            assert!(p_farm > 0.0, "{model:?}");
        }
    }

    #[test]
    fn frandsen_partial_overlap_beats_jensen_for_offset_turbine() {
        // A laterally offset turbine sits outside the Jensen top-hat cone
        // (factor 1.0) but still inside the Frandsen wake disk (partial
        // overlap), which is the model's designed-for regime.
        let ct = 0.8_f64;
        let t = turbine().with_thrust_coefficient(ct);
        let down = 240.0_f64; // 3D
        let offset = 40.0_f64; // rotor radius -> wake centerline still covers rotor
        let mut jensen = WindFarm::new(WakeModel::JensenPark, 90.0).with_wake_decay(0.05);
        jensen.push(0.0, 0.0, t.clone());
        jensen.push(down, offset, t.clone());
        let mut frandsen = WindFarm::new(WakeModel::Frandsen, 90.0).with_wake_decay(0.05);
        frandsen.push(0.0, 0.0, t.clone());
        frandsen.push(down, offset, t);
        let v_j = jensen.effective_wind_speeds(10.0);
        let v_f = frandsen.effective_wind_speeds(10.0);
        // Wake radius at 3D for Jensen: 40 + 0.05*240 = 52 > 40, so both
        // see a wake; assert Frandsen produces a *smaller* deficit because
        // the partial overlap weights it (deficit * fraction < full).
        let deficit_j = 1.0 - v_j[1] / 10.0;
        let deficit_f = 1.0 - v_f[1] / 10.0;
        assert!(deficit_f > 0.0, "frandsen must produce a deficit");
        assert!(
            deficit_f < deficit_j,
            "expected Frandsen deficit {deficit_f} < Jensen deficit {deficit_j} for offset rotor"
        );
    }

    #[test]
    fn eddy_viscosity_profile_smooths_with_distance() {
        let profile_close = EddyViscosityProfile::new(40.0, 1.0 - 0.2_f64.sqrt(), 0.05, 160.0);
        let profile_far = EddyViscosityProfile::new(40.0, 1.0 - 0.2_f64.sqrt(), 0.05, 960.0);
        // Centerline deficit decays downstream; wake spreads outwards.
        assert!(profile_far.deficit_at(0.0) < profile_close.deficit_at(0.0));
        assert!(profile_far.deficit_at(80.0) > 0.0);
        assert!(profile_close.deficit_at(120.0) < profile_close.deficit_at(0.0));
    }

    #[test]
    fn disk_overlap_fraction_cases() {
        assert!((disk_overlap_fraction(40.0, 40.0, 0.0) - 1.0).abs() < 1e-12);
        assert!((disk_overlap_fraction(40.0, 80.0, 0.0) - 1.0).abs() < 1e-12);
        assert!(disk_overlap_fraction(40.0, 40.0, 200.0).abs() < 1e-12);
        // Half-overlap: centers separated by exactly the rotor radius with
        // an equal-radius wake gives lens area fraction ~0.39.
        let half = disk_overlap_fraction(40.0, 40.0, 40.0);
        assert!(half > 0.3 && half < 0.5, "half overlap = {half}");
        // Contained wake smaller than the rotor scales with the area ratio.
        let contained = disk_overlap_fraction(40.0, 20.0, 0.0);
        assert!((contained - 0.25).abs() < 1e-12, "contained = {contained}");
    }
}
