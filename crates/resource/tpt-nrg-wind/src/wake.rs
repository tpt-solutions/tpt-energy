//! Wind farm wake-loss models.

use serde::{Deserialize, Serialize};

use crate::turbine::WindTurbine;

/// Wake model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WakeModel {
    /// Jensen / PARK model — top-hat wake with linear expansion.
    JensenPark,
    /// Frandsen model — Gaussian wake profile.
    Frandsen,
    /// Simple eddy-viscosity model.
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
    /// Wake decay coefficient `k` for the Jensen model (typically 0.05 for
    /// onshore, 0.04 for offshore).
    pub wake_decay_k: f64,
    /// Prevailing wind direction in degrees (the direction the wind is
    /// blowing toward). Each downstream turbine experiences a wake from
    /// every upstream turbine in the same wind line.
    pub wind_direction_deg: f64,
}

impl WindFarm {
    /// Construct a wind farm.
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
    pub fn with_wake_decay(mut self, k: f64) -> Self {
        self.wake_decay_k = k;
        self
    }

    /// Compute the effective wind speed at each turbine, applying wake
    /// losses from all upstream turbines.
    ///
    /// Only [`WakeModel::JensenPark`] is implemented; selecting an
    /// unimplemented model panics rather than silently computing a
    /// different model's result.
    pub fn effective_wind_speeds(&self, free_stream_mps: f64) -> Vec<f64> {
        assert!(
            self.wake_model == WakeModel::JensenPark,
            "WakeModel::{:?} is not implemented yet; use WakeModel::JensenPark",
            self.wake_model
        );
        let n = self.turbines.len();
        let mut v_eff = vec![free_stream_mps; n];
        let dir = self.wind_direction_rad();
        // Project each pair onto the wind direction
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                let (xi, yi, _) = self.turbines[i];
                let (xj, yj, ref upstream) = self.turbines[j];
                // Vector from j to i
                let dx = xi - xj;
                let dy = yi - yj;
                // Downstream if (i - j) projected on dir is positive
                let down = dx * dir.sin() + dy * dir.cos();
                if down <= 0.0 {
                    continue;
                }
                let lateral = (dx * dir.cos() - dy * dir.sin()).abs();
                let d = upstream.rotor_diameter_m;
                let r = d * 0.5;
                // Jensen / PARK wake: deficit = (1 - sqrt(1 - C_T)) * (D/(D+2k*down))^2,
                // using the upstream turbine's own thrust coefficient.
                let ct: f64 = upstream.thrust_coefficient;
                let dw = d + 2.0 * self.wake_decay_k * down;
                let deficit_axial = (1.0 - (1.0 - ct).sqrt()) * (d / dw).powi(2);
                // Top-hat wake with radius r_w = D/2 + k*down: full deficit
                // inside the wake cone, none outside.
                let wake_radius = r + self.wake_decay_k * down;
                if lateral >= wake_radius {
                    continue;
                }
                v_eff[i] *= 1.0 - deficit_axial;
            }
        }
        v_eff
    }

    /// Total farm power output (MW) at the given free-stream wind speed.
    pub fn total_power_output(&self, free_stream_mps: f64) -> f64 {
        let v_eff = self.effective_wind_speeds(free_stream_mps);
        let mut total = 0.0;
        for (i, (_, _, t)) in self.turbines.iter().enumerate() {
            total += t.power_at(v_eff[i]);
        }
        total
    }

    fn wind_direction_rad(&self) -> f64 {
        // Convention: "direction toward which the wind is blowing" in
        // degrees clockwise from north.
        self.wind_direction_deg.to_radians()
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
        let mut farm = WindFarm::new(WakeModel::JensenPark, 90.0);
        farm.push(0.0, 0.0, turbine());
        let v = farm.effective_wind_speeds(10.0);
        assert!((v[0] - 10.0).abs() < 1e-9);
    }

    #[test]
    fn downstream_turbine_slower() {
        let mut farm = WindFarm::new(WakeModel::JensenPark, 90.0).with_wake_decay(0.05);
        // Two turbines 5 diameters apart, wind blowing east (90°)
        farm.push(0.0, 0.0, turbine());
        farm.push(400.0, 0.0, turbine()); // 5D downstream
        let v = farm.effective_wind_speeds(10.0);
        // Upstream turbine: free stream = 10
        assert!((v[0] - 10.0).abs() < 1e-6);
        // Downstream turbine: should be slower due to wake
        assert!(v[1] < 10.0);
        assert!(v[1] > 5.0);
    }

    #[test]
    fn lateral_turbine_unaffected() {
        let mut farm = WindFarm::new(WakeModel::JensenPark, 90.0).with_wake_decay(0.05);
        // Two turbines well outside the wake cone
        farm.push(0.0, 0.0, turbine());
        farm.push(400.0, 500.0, turbine()); // 5D downstream, 6.25D lateral
        let v = farm.effective_wind_speeds(10.0);
        assert!((v[1] - 10.0).abs() < 1e-6);
    }

    #[test]
    fn total_power_loss_in_farm() {
        let mut farm = WindFarm::new(WakeModel::JensenPark, 90.0).with_wake_decay(0.05);
        // 4 turbines in a row, 5D apart
        for i in 0..4 {
            farm.push(i as f64 * 400.0, 0.0, turbine());
        }
        let p_farm = farm.total_power_output(12.0);
        let p_single = turbine().power_at(12.0);
        assert!(
            p_farm < 4.0 * p_single,
            "p_farm={p_farm}, 4*p={}",
            4.0 * p_single
        );
        assert!(p_farm > 0.0);
    }
}
