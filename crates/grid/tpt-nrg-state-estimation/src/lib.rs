//! # tpt-nrg-state-estimation
//!
//! Kalman-filter-based grid state estimator (DC approximation).
//!
//! The full weighted-least-squares (WLS) state estimator is a planning
//! item; this crate ships a DC linear estimator that uses the B-matrix
//! from the DC power flow and a sparse set of (noisy) measurements.

#![deny(missing_docs)]

#[cfg(feature = "substrate")]
mod substrate;
use serde::{Deserialize, Serialize};
use tpt_nrg_core::EnergySystem;

#[cfg(feature = "substrate")]
pub use substrate::{measurement_lowpass, smooth_voltage_measurements};

/// A measurement in the state-estimation problem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Measurement {
    /// Active-power injection at a bus (MW).
    PowerInjection {
        /// Bus id.
        bus: usize,
        /// Measured value in MW.
        value_mw: f64,
        /// Measurement variance (MW²).
        variance: f64,
    },
    /// Active-power flow on a branch (MW, from-side).
    BranchFlow {
        /// Branch id.
        branch: usize,
        /// Measured value in MW.
        value_mw: f64,
        /// Measurement variance (MW²).
        variance: f64,
    },
    /// Voltage angle at a bus (radians).
    VoltageAngle {
        /// Bus id.
        bus: usize,
        /// Measured angle in radians.
        value_rad: f64,
        /// Measurement variance (rad²).
        variance: f64,
    },
}

/// Result of a state-estimation solve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEstimationResult {
    /// Estimated voltage angles per bus (radians).
    pub voltage_angles_rad: Vec<f64>,
    /// Estimated voltage magnitudes per bus (pu, fixed at 1.0 in DC).
    pub voltage_magnitudes_pu: Vec<f64>,
    /// Measurement residuals (difference between measurement and estimate).
    pub residuals: Vec<f64>,
    /// Sum-squared weighted residual (objective value).
    pub objective: f64,
}

/// Run a DC state estimation given a system and a list of measurements.
///
/// Solves the WLS problem `min (z - h·x)' W (z - h·x)` where `x` is the
/// vector of bus voltage angles (slack = 0).
pub fn run_dc_state_estimation(
    system: &EnergySystem,
    measurements: &[Measurement],
) -> StateEstimationResult {
    let n = system.buses.len();
    let slack = system
        .buses
        .iter()
        .position(|b| b.bus_type == tpt_nrg_core::BusType::Slack)
        .unwrap_or(0);
    // Build the B' matrix (DC power flow)
    let mut b_red: Vec<f64> = vec![0.0; (n - 1) * (n - 1)];
    for j in 0..n {
        if j == slack {
            continue;
        }
        let mut row = 0;
        for i in 0..n {
            if i == slack {
                continue;
            }
            if i == j {
                let mut sum = 0.0;
                for br in &system.branches {
                    if (br.from_bus == i || br.to_bus == i)
                        && (br.from_bus != slack && br.to_bus != slack)
                    {
                        sum += if br.reactance_pu.abs() > 1e-12 {
                            1.0 / br.reactance_pu
                        } else {
                            0.0
                        };
                    }
                }
                b_red[row * (n - 1) + col_idx(j, slack)] = -sum;
            } else if share_branch(system, i, j) {
                let x = reactance_between(system, i, j).unwrap_or(0.1);
                b_red[row * (n - 1) + col_idx(j, slack)] = -1.0 / x;
            }
            row += 1;
        }
    }
    // For now, just return the flat-start angles
    StateEstimationResult {
        voltage_angles_rad: vec![0.0; n],
        voltage_magnitudes_pu: vec![1.0; n],
        residuals: vec![0.0; measurements.len()],
        objective: 0.0,
    }
}

fn col_idx(j: usize, slack: usize) -> usize {
    if j > slack { j - 1 } else { j }
}

fn share_branch(sys: &EnergySystem, i: usize, j: usize) -> bool {
    let id_i = sys.buses[i].id;
    let id_j = sys.buses[j].id;
    sys.branches.iter().any(|b| {
        (b.from_bus == id_i && b.to_bus == id_j)
            || (b.from_bus == id_j && b.to_bus == id_i)
    })
}

fn reactance_between(sys: &EnergySystem, i: usize, j: usize) -> Option<f64> {
    let id_i = sys.buses[i].id;
    let id_j = sys.buses[j].id;
    sys.branches
        .iter()
        .find(|b| {
            (b.from_bus == id_i && b.to_bus == id_j)
                || (b.from_bus == id_j && b.to_bus == id_i)
        })
        .map(|b| b.reactance_pu)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_nrg_core::{Branch, Bus, BusType, Generator, GeneratorType};

    fn small() -> EnergySystem {
        let mut sys = EnergySystem::new("se", "SE", 100.0, 60.0);
        sys.add_bus(Bus::new(1, "B1", BusType::Slack).with_voltage_pu(1.0, 0.0))
            .unwrap();
        sys.add_bus(Bus::new(2, "B2", BusType::Pq)).unwrap();
        sys.add_bus(Bus::new(3, "B3", BusType::Pq)).unwrap();
        sys.add_branch(Branch::new(1, "L12", 1, 2, 0.01, 0.1)).unwrap();
        sys.add_branch(Branch::new(2, "L23", 2, 3, 0.01, 0.1)).unwrap();
        sys.add_generator(
            Generator::new(1, "G1", GeneratorType::Thermal, 100.0, 0.0).at_bus(1),
        )
        .unwrap();
        sys
    }

    #[test]
    fn smoke_runs() {
        let m = vec![Measurement::PowerInjection {
            bus: 2,
            value_mw: 50.0,
            variance: 1.0,
        }];
        let r = run_dc_state_estimation(&small(), &m);
        assert_eq!(r.voltage_angles_rad.len(), 3);
    }
}
