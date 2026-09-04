//! # tpt-nrg-unit-commitment
//!
//! Unit commitment: decide which units to start up over a horizon to meet
//! demand at minimum cost, respecting min up/down times and startup costs.
//!
//! This implementation uses a priority-list / forward-dispatch heuristic
//! suitable for planning studies. A full MILP implementation will be
//! provided when the substrate MILP solver is available.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

use tpt_nrg_core::EnergySystem;

/// Unit-commitment result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitCommitmentResult {
    /// Per-unit per-interval commitment: `commitment[unit][t]` = 1 if on.
    pub commitment: Vec<Vec<u8>>,
    /// Per-unit per-interval output (MW).
    pub outputs: Vec<Vec<f64>>,
    /// Total cost over the horizon ($).
    pub total_cost_dollar: f64,
}

/// Solve a unit commitment using a priority-list heuristic.
///
/// Units are sorted by average full-load cost; we commit them in that
/// order until the load is met.
pub fn unit_commitment(
    system: &EnergySystem,
    load_profile_mw: &[f64],
) -> UnitCommitmentResult {
    let n = system.generators.len();
    let t = load_profile_mw.len();
    // Compute average cost for each unit
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        let ca = system.generators[a]
            .cost_curve
            .as_ref()
            .map(|c| {
                c.segments.iter().map(|s| s.incremental_cost_per_mwh).sum::<f64>()
                    / c.segments.len().max(1) as f64
            })
            .unwrap_or(100.0);
        let cb = system.generators[b]
            .cost_curve
            .as_ref()
            .map(|c| {
                c.segments.iter().map(|s| s.incremental_cost_per_mwh).sum::<f64>()
                    / c.segments.len().max(1) as f64
            })
            .unwrap_or(100.0);
        ca.partial_cmp(&cb).unwrap()
    });
    let mut commitment = vec![vec![0_u8; t]; n];
    let mut outputs = vec![vec![0.0_f64; t]; n];
    let mut total_cost = 0.0;
    for k in 0..t {
        let load = load_profile_mw[k];
        let mut remaining = load;
        for &i in &order {
            let g = &system.generators[i];
            if remaining <= 0.0 {
                break;
            }
            commitment[i][k] = 1;
            let output = remaining.min(g.p_max_mw).max(g.p_min_mw);
            outputs[i][k] = output;
            remaining -= output;
            if let Some(c) = &g.cost_curve {
                total_cost += c.cost_at(output);
            }
        }
    }
    UnitCommitmentResult {
        commitment,
        outputs,
        total_cost_dollar: total_cost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_nrg_core::{Bus, BusType, CostCurve, Generator, GeneratorType};

    fn sys() -> EnergySystem {
        let mut s = EnergySystem::new("uc", "UC", 100.0, 60.0);
        s.add_bus(Bus::new(1, "B1", BusType::Slack)).unwrap();
        s.add_bus(Bus::new(2, "B2", BusType::Pv)).unwrap();
        s.add_bus(Bus::new(3, "B3", BusType::Pv)).unwrap();
        s.add_generator(
            Generator::new(1, "G1", GeneratorType::Thermal, 100.0, 20.0)
                .at_bus(1)
                .with_cost_curve(CostCurve::piecewise(20.0, 100.0, 20.0, 20.0)),
        )
        .unwrap();
        s.add_generator(
            Generator::new(2, "G2", GeneratorType::Thermal, 80.0, 15.0)
                .at_bus(2)
                .with_cost_curve(CostCurve::piecewise(15.0, 80.0, 30.0, 30.0)),
        )
        .unwrap();
        s.add_generator(
            Generator::new(3, "G3", GeneratorType::Thermal, 50.0, 5.0)
                .at_bus(3)
                .with_cost_curve(CostCurve::piecewise(5.0, 50.0, 50.0, 50.0)),
        )
        .unwrap();
        s
    }

    #[test]
    fn serves_load() {
        let load = vec![50.0, 75.0, 100.0, 125.0];
        let r = unit_commitment(&sys(), &load);
        for k in 0..load.len() {
            let total: f64 = r.outputs.iter().map(|o| o[k]).sum();
            assert!(total >= load[k] - 1.0, "t={k}: total={total}, load={}", load[k]);
        }
    }

    #[test]
    fn dispatches_in_merit_order() {
        let load = vec![10.0];
        let r = unit_commitment(&sys(), &load);
        // For load 10 MW, the cheapest unit (G1) covers it; G2 and G3 are
        // not committed.
        assert_eq!(r.commitment[0][0], 1);
        assert_eq!(r.commitment[1][0], 0);
        assert_eq!(r.commitment[2][0], 0);
        let total: f64 = r.outputs.iter().map(|o| o[0]).sum();
        assert!(total >= load[0] - 1.0);
    }
}
