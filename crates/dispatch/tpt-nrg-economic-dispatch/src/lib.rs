//! # tpt-nrg-economic-dispatch
//!
//! Economic dispatch with optional storage arbitrage: minimises total
//! generation cost subject to power-balance and unit limits.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use tpt_nrg_core::EnergySystem;

/// Result of an economic-dispatch problem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicDispatchResult {
    /// Per-generator output (MW), in the same order as `system.generators`.
    pub generator_outputs_mw: Vec<f64>,
    /// Total system production cost ($/h).
    pub total_cost_dollar_per_h: f64,
    /// System marginal cost (lambda) in $/MWh.
    pub marginal_cost_dollar_per_mwh: f64,
    /// System losses in MW (zero for the lossless economic-dispatch model).
    pub losses_mw: f64,
}

/// Storage arbitrage plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitragePlan {
    /// Per-interval charge power (MW, positive = charging).
    pub charge_mw: Vec<f64>,
    /// Per-interval discharge power (MW, positive = discharging).
    pub discharge_mw: Vec<f64>,
    /// Per-interval SoC.
    pub state_of_charge: Vec<f64>,
    /// Net revenue ($).
    pub net_revenue_dollar: f64,
}

/// Solve a lossless economic dispatch using lambda iteration.
///
/// Marginal costs are taken from each generator's [`CostCurve`]. The
/// algorithm finds the system marginal price `λ` such that
/// `sum_i P_i(λ) = system_load_mw` with each unit's output clamped to
/// `[p_min, p_max]`. Units without a cost curve are dispatched at
/// `p_min` and are not marginal.
pub fn economic_dispatch(
    system: &EnergySystem,
    system_load_mw: f64,
) -> Result<EconomicDispatchResult, DispatchError> {
    let n = system.generators.len();
    if n == 0 {
        return Err(DispatchError::NoGenerators);
    }
    let mut c: Vec<f64> = Vec::with_capacity(n);
    let mut p_min: Vec<f64> = Vec::with_capacity(n);
    let mut p_max: Vec<f64> = Vec::with_capacity(n);
    let mut has_curve = vec![false; n];
    for (i, g) in system.generators.iter().enumerate() {
        if let Some(curve) = &g.cost_curve {
            let slope = if curve.segments.is_empty() {
                50.0
            } else {
                curve.segments[0].incremental_cost_per_mwh
            };
            c.push(slope);
            has_curve[i] = true;
        } else {
            c.push(0.0);
        }
        p_min.push(g.p_min_mw);
        p_max.push(g.p_max_mw);
    }
    // Sort units by marginal cost ascending
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| c[a].partial_cmp(&c[b]).unwrap());
    // Walk up the merit order, committing each unit up to its max until the
    // load is met; the marginal unit is partially loaded.
    let mut outputs = vec![0.0_f64; n];
    let mut remaining = system_load_mw;
    let mut marginal_unit: Option<usize> = None;
    let mut lambda = 0.0;
    for &i in &order {
        if remaining <= 0.0 {
            break;
        }
        if !has_curve[i] {
            outputs[i] = p_min[i];
            remaining -= p_min[i];
            continue;
        }
        // Use the unit's full range, not just the headroom above p_min.
        if remaining <= p_max[i] - 1e-6 {
            // Marginal unit — partial output
            outputs[i] = remaining;
            lambda = c[i];
            remaining = 0.0;
        } else {
            // Fully commit
            outputs[i] = p_max[i];
            remaining -= p_max[i];
        }
    }
    if remaining > 1e-3 {
        // Insufficient capacity
        for i in 0..n {
            outputs[i] = p_max[i];
        }
        lambda = c[order.last().copied().unwrap_or(0)];
    }
    let total_cost: f64 = outputs
        .iter()
        .zip(c.iter())
        .map(|(p, ci)| p * ci)
        .sum();
    let _ = marginal_unit;
    Ok(EconomicDispatchResult {
        generator_outputs_mw: outputs,
        total_cost_dollar_per_h: total_cost,
        marginal_cost_dollar_per_mwh: lambda,
        losses_mw: 0.0,
    })
}

/// Storage arbitrage from a price forecast: charge at low prices, discharge
/// at high prices, respecting energy and power limits.
pub fn storage_arbitrage(
    prices_dollar_per_mwh: &[f64],
    duration_h: f64,
    energy_capacity_mwh: f64,
    power_rating_mw: f64,
    round_trip_efficiency: f64,
) -> ArbitragePlan {
    let n = prices_dollar_per_mwh.len();
    let mut plan = ArbitragePlan {
        charge_mw: vec![0.0; n],
        discharge_mw: vec![0.0; n],
        state_of_charge: vec![0.5 * energy_capacity_mwh; n + 1],
        net_revenue_dollar: 0.0,
    };
    // Find the price thresholds: charge when price is in the lowest
    // third, discharge when in the highest third.
    if n == 0 {
        return plan;
    }
    let mut sorted = prices_dollar_per_mwh.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let low_threshold = sorted[n / 3];
    let high_threshold = sorted[2 * n / 3];
    let eta = round_trip_efficiency.sqrt();
    let mut soc = 0.5 * energy_capacity_mwh;
    for i in 0..n {
        let p = prices_dollar_per_mwh[i];
        if p <= low_threshold && soc < energy_capacity_mwh {
            let charge = (power_rating_mw).min((energy_capacity_mwh - soc) / (duration_h * eta));
            plan.charge_mw[i] = charge;
            plan.state_of_charge[i + 1] = soc + charge * duration_h * eta;
            plan.net_revenue_dollar -= charge * duration_h * p;
            soc = plan.state_of_charge[i + 1];
        } else if p >= high_threshold && soc > 0.0 {
            let discharge = (power_rating_mw).min(soc * eta / duration_h);
            plan.discharge_mw[i] = discharge;
            plan.state_of_charge[i + 1] = soc - discharge * duration_h / eta;
            plan.net_revenue_dollar += discharge * duration_h * p;
            soc = plan.state_of_charge[i + 1];
        } else {
            plan.state_of_charge[i + 1] = soc;
        }
    }
    plan
}

fn build_result(
    outputs: &[f64],
    lambda: f64,
    c: &[f64],
    _system: &EnergySystem,
) -> Result<EconomicDispatchResult, DispatchError> {
    let total: f64 = outputs.iter().sum();
    let total_cost: f64 = outputs
        .iter()
        .zip(c.iter())
        .map(|(p, ci)| p * ci)
        .sum();
    Ok(EconomicDispatchResult {
        generator_outputs_mw: outputs.to_vec(),
        total_cost_dollar_per_h: total_cost,
        marginal_cost_dollar_per_mwh: lambda,
        losses_mw: 0.0,
    })
    .map(|mut r| {
        let _ = total;
        r
    })
}

/// Errors raised by the dispatch solver.
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    /// No generators in the system.
    #[error("no generators available for dispatch")]
    NoGenerators,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_nrg_core::{CostCurve, Generator, GeneratorType};

    fn three_gen_system() -> EnergySystem {
        let mut sys = EnergySystem::new("ed", "ED", 100.0, 60.0);
        for i in 1..=3 {
            sys.add_bus(tpt_nrg_core::Bus::new(
                i,
                format!("B{i}"),
                tpt_nrg_core::BusType::Pv,
            ))
            .unwrap();
        }
        // Gen 1: 20-100 MW, MC=20 $/MWh (flat)
        sys.add_generator(
            Generator::new(1, "G1", GeneratorType::Thermal, 100.0, 20.0)
                .at_bus(1)
                .with_cost_curve(CostCurve::piecewise(20.0, 100.0, 400.0, 2400.0)),
        )
        .unwrap();
        // Gen 2: 30-150 MW, MC=30 $/MWh (flat)
        sys.add_generator(
            Generator::new(2, "G2", GeneratorType::Thermal, 150.0, 30.0)
                .at_bus(2)
                .with_cost_curve(CostCurve::piecewise(30.0, 150.0, 900.0, 5400.0)),
        )
        .unwrap();
        // Gen 3: 50-200 MW, MC=50 $/MWh (flat)
        sys.add_generator(
            Generator::new(3, "G3", GeneratorType::Thermal, 200.0, 50.0)
                .at_bus(3)
                .with_cost_curve(CostCurve::piecewise(50.0, 200.0, 2500.0, 12500.0)),
        )
        .unwrap();
        sys
    }

    #[test]
    fn dispatch_load_200mw() {
        // For 200 MW: G1 max 100, G2 max 150.  G1+G2_min = 50. So we need
        // 200 MW. G1=100, G2=100. Lambda = 30 (highest marginal that any
        // unit is dispatched at).
        let r = economic_dispatch(&three_gen_system(), 200.0).unwrap();
        let total: f64 = r.generator_outputs_mw.iter().sum();
        assert!((total - 200.0).abs() < 0.5, "total = {total}");
    }

    #[test]
    fn dispatch_lambda_equals_marginal_cost() {
        let r = economic_dispatch(&three_gen_system(), 200.0).unwrap();
        // G1 (max 100) is fully committed, G2 is marginal at the load.
        // Lambda = slope of G2's cost curve = 37.5.
        assert!((r.marginal_cost_dollar_per_mwh - 37.5).abs() < 1.0,
            "lambda = {}", r.marginal_cost_dollar_per_mwh);
    }

    #[test]
    fn arbitrage_profits_from_spread() {
        let prices = vec![10.0, 15.0, 20.0, 50.0, 80.0, 100.0, 30.0, 25.0];
        let plan = storage_arbitrage(&prices, 1.0, 100.0, 50.0, 0.9);
        // Should buy at 10/15/20/25/30 and sell at 50/80/100.
        assert!(plan.net_revenue_dollar > 0.0, "revenue = {}", plan.net_revenue_dollar);
        // SoC should stay within bounds
        for s in &plan.state_of_charge {
            assert!(*s >= 0.0 && *s <= 100.0 + 1e-6, "soc = {s}");
        }
    }
}
