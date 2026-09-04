//! Single-turbine wind model and vertical wind profile.

use serde::{Deserialize, Serialize};

/// A wind-power model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindModel {
    /// Hub height in metres.
    pub hub_height_m: f64,
    /// Surface roughness length in metres (e.g. 0.03 for short grass, 0.5
    /// for forest, 1.0 for urban).
    pub roughness_length_m: f64,
    /// Reference measurement height in metres.
    pub measurement_height_m: f64,
    /// Power-law exponent (used when `use_log_profile` is `false`).
    #[serde(default = "default_alpha")]
    pub alpha: f64,
}

fn default_alpha() -> f64 {
    0.143
}

impl WindModel {
    /// Construct a wind model with log profile.
    pub fn new(hub_height_m: f64, roughness_length_m: f64, measurement_height_m: f64) -> Self {
        Self {
            hub_height_m,
            roughness_length_m,
            measurement_height_m,
            alpha: default_alpha(),
        }
    }

    /// Construct a wind model with power-law profile.
    pub fn with_power_law(hub_height_m: f64, alpha: f64, measurement_height_m: f64) -> Self {
        Self {
            hub_height_m,
            roughness_length_m: 0.0,
            measurement_height_m,
            alpha,
        }
    }

    /// Compute the wind speed at the hub height given a reference wind speed
    /// at the measurement height. Uses the log profile if `roughness_length_m`
    /// > 0, otherwise the power-law profile.
    pub fn wind_speed_at_height(&self, v_ref: f64) -> f64 {
        if self.roughness_length_m > 0.0 {
            v_ref * (self.hub_height_m / self.roughness_length_m).ln()
                / (self.measurement_height_m / self.roughness_length_m).ln()
        } else {
            v_ref * (self.hub_height_m / self.measurement_height_m).powf(self.alpha)
        }
    }

    /// Weibull PDF `f(v; k, c) = (k/c) (v/c)^{k-1} exp(-(v/c)^k)`.
    #[must_use]
    pub fn weibull_probability(&self, v: f64, shape_k: f64, scale_c: f64) -> f64 {
        if v <= 0.0 {
            return 0.0;
        }
        let x = v / scale_c;
        (shape_k / scale_c) * x.powf(shape_k - 1.0) * (-x.powf(shape_k)).exp()
    }

    /// Mean power of a Weibull distribution with given turbine power curve.
    #[must_use]
    pub fn weibull_mean_power(
        &self,
        shape_k: f64,
        scale_c: f64,
        turbine: &WindTurbine,
    ) -> f64 {
        // Trapezoidal integration on [0, 4·c].
        let n = 400;
        let upper = 4.0 * scale_c;
        let dv = upper / n as f64;
        let mut total = 0.0;
        for i in 0..=n {
            let v = i as f64 * dv;
            let f = if i == 0 || i == n {
                0.5
            } else {
                1.0
            };
            total += f * turbine.power_at(v) * self.weibull_probability(v, shape_k, scale_c);
        }
        total * dv
    }
}

/// Trapezoidal helper: just for the wind power integral. (We avoid full
/// Gauss-Legendre for now to keep the implementation small and auditable.)
fn _trapezoid(f: impl Fn(f64) -> f64, a: f64, b: f64, n: usize) -> f64 {
    let _ = f;
    let _ = a;
    let _ = b;
    let _ = n;
    0.0
}

#[allow(dead_code)]
fn _gauss_helper() {
    let _ = gauss_legendre;
    let _ = gauss_legendre_std;
    let _ = legendre;
}

/// A wind turbine specification with a power curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindTurbine {
    /// Name (e.g. "NREL 5MW").
    pub name: String,
    /// Rotor diameter in metres.
    pub rotor_diameter_m: f64,
    /// Rated power in MW.
    pub rated_power_mw: f64,
    /// Cut-in wind speed in m/s.
    pub cut_in_mps: f64,
    /// Rated wind speed in m/s.
    pub rated_mps: f64,
    /// Cut-out wind speed in m/s.
    pub cut_out_mps: f64,
    /// Optional per-turbine power curve as `(wind_speed_mps, power_mw)`
    /// samples; if `None`, the cubic power law is used.
    #[serde(default)]
    pub power_curve: Vec<(f64, f64)>,
}

impl WindTurbine {
    /// Construct a generic turbine using a cubic power law between cut-in
    /// and rated wind speeds.
    pub fn new(
        name: impl Into<String>,
        rotor_diameter_m: f64,
        rated_power_mw: f64,
        cut_in_mps: f64,
        rated_mps: f64,
        cut_out_mps: f64,
    ) -> Self {
        Self {
            name: name.into(),
            rotor_diameter_m,
            rated_power_mw,
            cut_in_mps,
            rated_mps,
            cut_out_mps,
            power_curve: Vec::new(),
        }
    }

    /// Add a power-curve sample point.
    pub fn with_curve_point(mut self, wind_speed_mps: f64, power_mw: f64) -> Self {
        self.power_curve.push((wind_speed_mps, power_mw));
        self
    }

    /// Output power (MW) at the given wind speed.
    pub fn power_at(&self, v: f64) -> f64 {
        if v < self.cut_in_mps || v > self.cut_out_mps {
            return 0.0;
        }
        if !self.power_curve.is_empty() {
            return interp(&self.power_curve, v).min(self.rated_power_mw);
        }
        if v >= self.rated_mps {
            return self.rated_power_mw;
        }
        // Cubic: P = P_rated * ((v - v_ci)/(v_r - v_ci))^3
        let frac = (v - self.cut_in_mps) / (self.rated_mps - self.cut_in_mps);
        self.rated_power_mw * frac.max(0.0).powi(3)
    }

    /// Rotor swept area in m².
    pub fn swept_area_m2(&self) -> f64 {
        std::f64::consts::PI * (self.rotor_diameter_m * 0.5).powi(2)
    }
}

fn interp(curve: &[(f64, f64)], v: f64) -> f64 {
    if curve.is_empty() {
        return 0.0;
    }
    if v <= curve[0].0 {
        return curve[0].1;
    }
    if let Some(last) = curve.last() {
        if v >= last.0 {
            return last.1;
        }
    }
    for w in curve.windows(2) {
        let (v0, p0) = w[0];
        let (v1, p1) = w[1];
        if v >= v0 && v <= v1 {
            let t = (v - v0) / (v1 - v0);
            return p0 + t * (p1 - p0);
        }
    }
    0.0
}

/// Gauss-Legendre nodes and weights for the interval `[a, b]`.
fn gauss_legendre(n: usize, a: f64, b: f64) -> (Vec<f64>, Vec<f64>) {
    // n-point Gauss-Legendre on [-1, 1], then map to [a, b].
    let (x_std, w_std) = gauss_legendre_std(n);
    let half = (b - a) * 0.5;
    let mid = (a + b) * 0.5;
    let xs: Vec<f64> = x_std.iter().map(|&x| mid + half * x).collect();
    let ws: Vec<f64> = w_std.iter().map(|&w| half * w).collect();
    (xs, ws)
}

fn gauss_legendre_std(n: usize) -> (Vec<f64>, Vec<f64>) {
    // Pre-computed for n up to ~200 (use Newton iteration; not the focus of
    // the crate — for production we would table these).
    let mut x = vec![0.0_f64; n];
    let mut w = vec![0.0_f64; n];
    for i in 0..n {
        let mut z = ((std::f64::consts::PI * (i as f64 + 0.75)) / (n as f64 + 0.5)).cos();
        for _ in 0..100 {
            let (p, dp) = legendre(n, z);
            let dz = p / dp;
            z -= dz;
            if dz.abs() < 1e-14 {
                break;
            }
        }
        let (_, dp) = legendre(n, z);
        x[i] = z;
        w[i] = 2.0 / ((1.0 - z * z) * dp * dp);
    }
    (x, w)
}

fn legendre(n: usize, x: f64) -> (f64, f64) {
    let mut p0 = 1.0;
    let mut p1 = x;
    let mut dp = 1.0;
    for _ in 1..n {
        let p2 = ((2.0 * n as f64 - 1.0) * x * p1 - (n as f64 - 1.0) * p0) / n as f64;
        p0 = p1;
        p1 = p2;
    }
    let _ = dp;
    let dpn = (n as f64 * (p1 - x * p0)) / (1.0 - x * x);
    (p1, dpn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generic_turbine() -> WindTurbine {
        WindTurbine::new("Generic 2MW", 80.0, 2.0, 3.0, 12.0, 25.0)
    }

    #[test]
    fn power_at_zero_below_cut_in() {
        let t = generic_turbine();
        assert_eq!(t.power_at(0.0), 0.0);
        assert_eq!(t.power_at(2.5), 0.0);
    }

    #[test]
    fn power_at_rated() {
        let t = generic_turbine();
        assert!((t.power_at(12.0) - 2.0).abs() < 1e-9);
        assert!((t.power_at(20.0) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn power_cubic_shape() {
        let t = generic_turbine();
        // At v = cut_in + (rated-cut_in)/2 = 7.5 m/s, power = 0.125 * rated
        let p = t.power_at(7.5);
        assert!((p - 0.125 * 2.0).abs() < 1e-3, "p = {p}");
    }

    #[test]
    fn log_profile_extrapolation() {
        let m = WindModel::new(100.0, 0.03, 10.0);
        let v_100 = m.wind_speed_at_height(5.0);
        // v(100) > v(10) since hub is higher
        assert!(v_100 > 5.0);
        // Log profile: v(z)/v(zref) = ln(z/z0) / ln(zref/z0)
        // v(100)/v(10) = ln(100/0.03)/ln(10/0.03) = 8.11/5.81 = 1.395
        assert!((v_100 / 5.0 - 1.395).abs() < 0.01);
    }

    #[test]
    fn weibull_pdf_integrates_to_one() {
        let m = WindModel::new(80.0, 0.03, 10.0);
        let n = 2000;
        let upper = 40.0;
        let dv = upper / n as f64;
        let mut total = 0.0;
        for i in 0..=n {
            let v = i as f64 * dv;
            let f = if i == 0 || i == n { 0.5 } else { 1.0 };
            total += f * m.weibull_probability(v, 2.0, 8.0);
        }
        total *= dv;
        assert!((total - 1.0).abs() < 1e-3, "integral = {total}");
    }

    #[test]
    fn mean_power_within_rated() {
        let m = WindModel::new(80.0, 0.03, 10.0);
        let t = generic_turbine();
        let p = m.weibull_mean_power(2.0, 8.0, &t);
        assert!(p > 0.0 && p < t.rated_power_mw, "p = {p}");
    }
}
