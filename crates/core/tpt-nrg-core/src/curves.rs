//! Cost, heat-rate, and power-curve supporting types.

use serde::{Deserialize, Serialize};

/// A piecewise-linear cost curve: `cost($/h) = f(p_mw)`.
///
/// Each segment describes the incremental cost in $/MWh over the segment's MW
/// range, starting from `start_mw` up to (but not including) `end_mw`. The
/// intercept is in $/h, evaluated at zero output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostCurve {
    /// Optional fixed cost ($/h) incurred whenever the unit is online.
    #[serde(default)]
    pub no_load_cost: f64,
    /// Startup cost in $.
    #[serde(default)]
    pub startup_cost: f64,
    /// Sorted list of cost segments, by ascending MW.
    pub segments: Vec<CostSegment>,
}

/// A single segment of a piecewise-linear cost curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSegment {
    /// Lower MW bound of this segment (inclusive).
    pub start_mw: f64,
    /// Upper MW bound of this segment (exclusive).
    pub end_mw: f64,
    /// Marginal cost in $/MWh over this segment.
    pub incremental_cost_per_mwh: f64,
}

impl CostCurve {
    /// Build a simple two-segment piecewise-linear cost curve from a minimum
    /// output, a maximum output, and the marginal cost at and above the
    /// minimum. Useful for canonical test cases.
    pub fn piecewise(p_min: f64, p_max: f64, cost_at_min: f64, cost_at_max: f64) -> Self {
        let slope = (cost_at_max - cost_at_min) / (p_max - p_min);
        Self {
            no_load_cost: 0.0,
            startup_cost: 0.0,
            segments: vec![CostSegment {
                start_mw: p_min,
                end_mw: p_max,
                incremental_cost_per_mwh: slope,
            }],
        }
    }

    /// Evaluate the cost ($/h) at the given MW output. Returns `0.0` outside
    /// the defined range.
    pub fn cost_at(&self, p_mw: f64) -> f64 {
        let mut total = 0.0;
        for seg in &self.segments {
            if p_mw <= seg.start_mw {
                break;
            }
            let upper = p_mw.min(seg.end_mw);
            total += (upper - seg.start_mw) * seg.incremental_cost_per_mwh;
            if p_mw <= seg.end_mw {
                break;
            }
        }
        total
    }

    /// Evaluate the marginal cost ($/MWh) at the given MW output.
    pub fn marginal_cost_at(&self, p_mw: f64) -> f64 {
        for seg in &self.segments {
            if p_mw >= seg.start_mw && p_mw < seg.end_mw {
                return seg.incremental_cost_per_mwh;
            }
        }
        if let Some(last) = self.segments.last() {
            if p_mw >= last.end_mw {
                return last.incremental_cost_per_mwh;
            }
        }
        0.0
    }
}

/// A heat-rate curve: input fuel energy (MMBtu/h) as a function of output
/// electric power (MW).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatRateCurve {
    /// Points `(p_mw, heat_input_mmbtu_per_h)` sorted by ascending MW.
    pub points: Vec<(f64, f64)>,
}

impl HeatRateCurve {
    /// Construct an empty curve.
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    /// Add a sample point.
    pub fn push(mut self, p_mw: f64, heat_input_mmbtu_per_h: f64) -> Self {
        self.points.push((p_mw, heat_input_mmbtu_per_h));
        self
    }

    /// Linearly interpolate the heat input at `p_mw`. Returns `0.0` if the
    /// curve is empty.
    pub fn heat_input_at(&self, p_mw: f64) -> f64 {
        if self.points.is_empty() {
            return 0.0;
        }
        if p_mw <= self.points[0].0 {
            return self.points[0].1;
        }
        if let Some(last) = self.points.last() {
            if p_mw >= last.0 {
                return last.1;
            }
        }
        for w in self.points.windows(2) {
            let (p0, h0) = w[0];
            let (p1, h1) = w[1];
            if p_mw >= p0 && p_mw <= p1 {
                let t = (p_mw - p0) / (p1 - p0);
                return h0 + t * (h1 - h0);
            }
        }
        0.0
    }
}

impl Default for HeatRateCurve {
    fn default() -> Self {
        Self::new()
    }
}

/// A power curve: output power (MW) as a function of an input variable
/// (typically wind speed in m/s for wind, irradiance in W/m² for solar).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerCurve {
    /// Input variable name (e.g. `"wind_speed_mps"` or `"poa_w_per_m2"`).
    pub input: String,
    /// Output units (typically `"MW"`).
    pub output: String,
    /// Sample points sorted by ascending input.
    pub points: Vec<(f64, f64)>,
}

impl PowerCurve {
    /// Construct a new curve with the given input/output axis names.
    pub fn new(input: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            output: output.into(),
            points: Vec::new(),
        }
    }

    /// Add a sample point.
    pub fn push(mut self, input: f64, output: f64) -> Self {
        self.points.push((input, output));
        self
    }

    /// Linearly interpolate the output at the given input value. Saturates
    /// at the first / last sample outside the defined range.
    pub fn output_at(&self, input: f64) -> f64 {
        if self.points.is_empty() {
            return 0.0;
        }
        if input <= self.points[0].0 {
            return self.points[0].1;
        }
        if let Some(last) = self.points.last() {
            if input >= last.0 {
                return last.1;
            }
        }
        for w in self.points.windows(2) {
            let (i0, o0) = w[0];
            let (i1, o1) = w[1];
            if input >= i0 && input <= i1 {
                let t = (input - i0) / (i1 - i0);
                return o0 + t * (o1 - o0);
            }
        }
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_curve_piecewise() {
        let c = CostCurve::piecewise(10.0, 100.0, 20.0, 50.0);
        // At 10 MW: 0.  At 100 MW: (100-10) * (50-20)/(100-10) = 30.
        assert!((c.cost_at(10.0) - 0.0).abs() < 1e-9);
        assert!((c.cost_at(100.0) - 30.0).abs() < 1e-9);
        // Marginal cost is the slope.
        assert!((c.marginal_cost_at(50.0) - (50.0 - 20.0) / (100.0 - 10.0)).abs() < 1e-9);
    }

    #[test]
    fn heat_rate_interpolation() {
        let h = HeatRateCurve::new()
            .push(0.0, 100.0)
            .push(50.0, 400.0)
            .push(100.0, 800.0);
        assert!((h.heat_input_at(25.0) - 250.0).abs() < 1e-9);
        assert!((h.heat_input_at(75.0) - 600.0).abs() < 1e-9);
    }

    #[test]
    fn power_curve_interpolation() {
        let p = PowerCurve::new("wind_speed_mps", "MW")
            .push(0.0, 0.0)
            .push(5.0, 1.0)
            .push(15.0, 2.0)
            .push(25.0, 0.0);
        assert!((p.output_at(2.5) - 0.5).abs() < 1e-9);
        assert!((p.output_at(20.0) - 1.0).abs() < 1e-9);
    }
}
