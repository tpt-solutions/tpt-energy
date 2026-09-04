//! # tpt-nrg-lcoe
//!
//! Levelized cost of energy (LCOE), net present value (NPV), and internal
//! rate of return (IRR) for energy projects.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// Inputs to an LCOE calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcoeInputs {
    /// Total overnight capital expenditure in dollars.
    pub capex_dollar: f64,
    /// Annual fixed O&M cost in dollars per year.
    pub annual_fixed_om_dollar: f64,
    /// Variable O&M cost in dollars per MWh.
    pub variable_om_dollar_per_mwh: f64,
    /// Annual fuel cost in dollars per MWh.
    pub fuel_cost_dollar_per_mwh: f64,
    /// Annual energy production in MWh/year.
    pub annual_energy_mwh: f64,
    /// Project lifetime in years.
    pub lifetime_years: u32,
    /// Annual discount rate (e.g. 0.07 for 7%).
    pub discount_rate: f64,
    /// Capacity factor (0-1).
    pub capacity_factor: f64,
}

impl LcoeInputs {
    /// Construct a simplified LCOE input for a generic project.
    pub fn new(
        capex_dollar: f64,
        annual_fixed_om_dollar: f64,
        variable_om_dollar_per_mwh: f64,
        fuel_cost_dollar_per_mwh: f64,
        annual_energy_mwh: f64,
        lifetime_years: u32,
        discount_rate: f64,
        capacity_factor: f64,
    ) -> Self {
        Self {
            capex_dollar,
            annual_fixed_om_dollar,
            variable_om_dollar_per_mwh,
            fuel_cost_dollar_per_mwh,
            annual_energy_mwh,
            lifetime_years,
            discount_rate,
            capacity_factor,
        }
    }
}

/// Compute the levelized cost of energy in $/MWh.
///
/// `LCOE = (sum_t (CAPEX_t + O&M_t + fuel_t) / (1+r)^t) / (sum_t E_t / (1+r)^t)`
pub fn levelized_cost_of_energy(inputs: &LcoeInputs) -> f64 {
    let n = inputs.lifetime_years as i32;
    let r = inputs.discount_rate;
    let annuity_factor = if r.abs() < 1e-12 {
        n as f64
    } else {
        (1.0 - (1.0 + r).powi(-n)) / r
    };
    // Capital recovery factor: spreads CAPEX over the lifetime, discounted.
    let crf = if r.abs() < 1e-12 {
        1.0 / n as f64
    } else {
        r * (1.0 + r).powi(n) / ((1.0 + r).powi(n) - 1.0)
    };
    let annualized_capex = inputs.capex_dollar * crf;
    let annualized_fixed_om = inputs.annual_fixed_om_dollar;
    let per_mwh = inputs.variable_om_dollar_per_mwh + inputs.fuel_cost_dollar_per_mwh;
    let numerator = (annualized_capex + annualized_fixed_om) * annuity_factor
        + per_mwh * inputs.annual_energy_mwh * annuity_factor;
    let denominator = inputs.annual_energy_mwh * annuity_factor;
    if denominator == 0.0 {
        return 0.0;
    }
    numerator / denominator
}

/// Net present value of a series of cash flows (length n+1) at the given
/// discount rate. `cash_flows[0]` is the initial investment (typically
/// negative).
pub fn net_present_value(cash_flows: &[f64], discount_rate: f64) -> f64 {
    cash_flows
        .iter()
        .enumerate()
        .map(|(t, cf)| cf / (1.0 + discount_rate).powi(t as i32))
        .sum()
}

/// Internal rate of return: the discount rate that makes NPV zero.
///
/// Uses bisection; returns `None` if it cannot be bracketed in
/// `[-0.99, 10.0]`.
pub fn internal_rate_of_return(cash_flows: &[f64]) -> Option<f64> {
    let npv = |r: f64| net_present_value(cash_flows, r);
    let mut lo = -0.99_f64;
    let mut hi = 10.0_f64;
    let f_lo = npv(lo);
    let f_hi = npv(hi);
    if f_lo.signum() == f_hi.signum() {
        return None;
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        let f_mid = npv(mid);
        if f_mid.abs() < 1e-6 {
            return Some(mid);
        }
        if f_mid.signum() == f_lo.signum() {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(0.5 * (lo + hi))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lcoe_simple() {
        // CAPEX $1B, 30y, 7% discount, 100 MW at 50% CF = 100 * 0.5 * 8760 = 438 GWh/yr
        // Annual fixed O&M $10M, no fuel/variable.
        let i = LcoeInputs::new(
            1.0e9,
            1.0e7,
            0.0,
            0.0,
            100.0 * 0.5 * 8760.0,
            30,
            0.07,
            0.5,
        );
        let lcoe = levelized_cost_of_energy(&i);
        // Expected ~$200/MWh: 1B CAPEX over 30y at 7% CRF ≈ $80M/yr, plus
        // $10M O&M = $90M/yr / 438 GWh = $206/MWh.
        assert!(lcoe > 150.0 && lcoe < 250.0, "lcoe = {lcoe}");
    }

    #[test]
    fn npv_known_case() {
        // $1000 today, $1100 next year, 10% discount → NPV = 0
        let cf = vec![-1000.0, 1100.0];
        let npv = net_present_value(&cf, 0.10);
        assert!(npv.abs() < 1e-6, "npv = {npv}");
    }

    #[test]
    fn irr_known_case() {
        // -1000 today, +1100 next year → IRR = 10%
        let cf = vec![-1000.0, 1100.0];
        let irr = internal_rate_of_return(&cf).unwrap();
        assert!((irr - 0.10).abs() < 1e-3, "irr = {irr}");
    }
}
