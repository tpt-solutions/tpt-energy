//! DC power flow: linearized active-power-only flow.

use tpt_nrg_core::{BusType, EnergySystem};
use tpt_nrg_topology::{AdmittanceMatrix, AdmittanceMatrixBuilder};

use crate::result::{BranchFlow, PowerFlowResult};
use crate::solver::PowerFlowError;
use crate::util::{bus_index_map, bus_schedules_pu, find_slack};

/// Solve the DC power flow: `P = B'·θ` where `B'` is the negative of the
/// susceptance matrix with the slack row/column removed.
pub fn solve(system: &EnergySystem) -> Result<PowerFlowResult, PowerFlowError> {
    system.validate().map_err(map_validate)?;
    let n = system.buses.len();
    if n == 0 {
        return Err(PowerFlowError::NoSlackBus);
    }
    let slack = find_slack(system)?;

    let y_bus = AdmittanceMatrixBuilder::new(system).build();
    let (p_sched, _) = bus_schedules_pu(system);

    let (b_reduced, n_reduced) = build_b_reduced(&y_bus, n, slack);

    // RHS: p_sched excluding slack
    let rhs: Vec<f64> = p_sched
        .iter()
        .enumerate()
        .filter(|&(idx, _)| idx != slack)
        .map(|(_, &p)| p)
        .collect();
    let theta_reduced = solve_dense(n_reduced, &b_reduced, &rhs);

    // Place the angles back (slack keeps its 0 reference angle).
    let mut theta = vec![0.0_f64; n];
    let non_slack_slots: Vec<&mut f64> = theta
        .iter_mut()
        .enumerate()
        .filter(|&(idx, _)| idx != slack)
        .map(|(_, angle)| angle)
        .collect();
    // Lengths match by construction: both exclude the slack bus.
    for (angle, reduced_angle) in non_slack_slots.into_iter().zip(theta_reduced) {
        *angle = reduced_angle;
    }

    // V = schedule for generator buses, 1.0 pu otherwise (DC power flow).
    let voltages: Vec<f64> = (0..n)
        .map(|idx| match system.buses[idx].bus_type {
            BusType::Slack | BusType::Pv => system.buses[idx].voltage_magnitude_pu,
            _ => 1.0,
        })
        .collect();

    let map = bus_index_map(system);
    let flows = dc_branch_flows(system, &map, &theta);
    let slack_p_pu: f64 = p_sched
        .iter()
        .enumerate()
        .filter(|&(idx, _)| idx != slack)
        .map(|(_, &p)| -p)
        .sum();

    let base = system.base_mva;
    let mut gen_p = vec![0.0_f64; system.generators.len()];
    for (unit, gen) in system.generators.iter().enumerate() {
        if !gen.in_service {
            continue;
        }
        if let Some(Some(bus_idx)) = map.get(gen.bus_id) {
            gen_p[unit] = if *bus_idx == slack {
                slack_p_pu * base
            } else {
                p_sched[*bus_idx] * base
            };
        }
    }
    let gen_q = vec![0.0_f64; system.generators.len()];

    Ok(PowerFlowResult {
        converged: true,
        iterations: 1,
        final_mismatch: 0.0,
        bus_voltage_magnitude_pu: voltages,
        bus_voltage_angle_rad: theta,
        branch_flows: flows,
        total_losses_mw: 0.0,
        total_losses_mvar: 0.0,
        generator_p_mw: gen_p,
        generator_q_mvar: gen_q,
    })
}

/// Build the reduced `B'` susceptance matrix (slack row/column removed).
///
/// `B'[i,j] = -B[i,j]` for off-diagonals and `-B[i,i]` for the diagonal;
/// the diagonal must be the full row sum, including the removed slack
/// column — excluding it silently corrupts every angle in systems with
/// more than two buses.
fn build_b_reduced(y_bus: &AdmittanceMatrix, n: usize, slack: usize) -> (Vec<f64>, usize) {
    let n_reduced = n - 1;
    let mut b_reduced = vec![0.0_f64; n_reduced * n_reduced];
    let mut col = 0;
    for j in 0..n {
        if j == slack {
            continue;
        }
        let mut row = 0;
        for i in 0..n {
            if i == slack {
                continue;
            }
            b_reduced[row * n_reduced + col] = if i == j {
                -y_bus.b_ij(i, i)
            } else {
                -y_bus.b_ij(i, j)
            };
            row += 1;
        }
        col += 1;
    }
    (b_reduced, n_reduced)
}

/// Branch flows from DC angles: `P_ij = (θ_i - θ_j) / x_ij`, scaled to MW.
/// DC ignores losses (`P_ij + P_ji = 0` for every branch).
fn dc_branch_flows(system: &EnergySystem, map: &[Option<usize>], theta: &[f64]) -> Vec<BranchFlow> {
    let base = system.base_mva;
    system
        .branches
        .iter()
        .map(|branch| {
            let from_idx = map.get(branch.from_bus).and_then(|x| *x);
            let to_idx = map.get(branch.to_bus).and_then(|x| *x);
            let (p_from_mw, p_to_mw) = match (from_idx, to_idx) {
                (Some(from_idx), Some(to_idx)) if branch.reactance_pu.abs() >= 1e-12 => {
                    let p_pu = (theta[from_idx] - theta[to_idx]) / branch.reactance_pu;
                    let p_mw = p_pu * base;
                    (p_mw, -p_mw)
                }
                _ => (0.0, 0.0),
            };
            let loading = if branch.rating_mva > 0.0 {
                p_from_mw.abs() / branch.rating_mva
            } else {
                0.0
            };
            BranchFlow {
                id: branch.id,
                p_from_mw,
                q_from_mvar: 0.0,
                p_to_mw,
                q_to_mvar: 0.0,
                loading_fraction: loading,
            }
        })
        .collect()
}

fn map_validate(e: tpt_nrg_core::CoreError) -> PowerFlowError {
    match e {
        tpt_nrg_core::CoreError::NoSlackBus => PowerFlowError::NoSlackBus,
        tpt_nrg_core::CoreError::MultipleSlackBuses(n) => PowerFlowError::MultipleSlackBuses(n),
        other => PowerFlowError::Other(other.to_string()),
    }
}

/// Dense Gaussian elimination with partial pivoting.
fn solve_dense(n: usize, a_mat: &[f64], rhs: &[f64]) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    let mut a = a_mat.to_vec();
    let mut x = rhs.to_vec();
    for pivot_col in 0..n {
        // Partial pivoting: move the largest remaining entry in the column
        // to the diagonal.
        let (max_row, _) = (pivot_col..n)
            .map(|row| (row, a[row * n + pivot_col].abs()))
            .fold(
                (pivot_col, a[pivot_col * n + pivot_col].abs()),
                |(best_row, best_val), (row, val)| {
                    if val > best_val {
                        (row, val)
                    } else {
                        (best_row, best_val)
                    }
                },
            );
        if max_row != pivot_col {
            for col in 0..n {
                a.swap(pivot_col * n + col, max_row * n + col);
            }
            x.swap(pivot_col, max_row);
        }
        let pivot = a[pivot_col * n + pivot_col];
        if pivot.abs() < 1e-14 {
            continue;
        }
        for row in (pivot_col + 1)..n {
            let factor = a[row * n + pivot_col] / pivot;
            for col in pivot_col..n {
                a[row * n + col] -= factor * a[pivot_col * n + col];
            }
            x[row] -= factor * x[pivot_col];
        }
    }
    for row in (0..n).rev() {
        let mut sum = x[row];
        for col in (row + 1)..n {
            sum -= a[row * n + col] * x[col];
        }
        let diag = a[row * n + row];
        x[row] = if diag.abs() > 1e-14 { sum / diag } else { 0.0 };
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_nrg_core::{Branch, Bus, BusType, EnergySystem, Generator, GeneratorType};

    fn two_bus() -> EnergySystem {
        let mut sys = EnergySystem::new("two", "Two-Bus", 100.0, 60.0);
        sys.add_bus(Bus::new(1, "Slack", BusType::Slack).with_voltage_pu(1.05, 0.0))
            .unwrap();
        sys.add_bus(Bus::new(2, "PQ", BusType::Pq).with_load(50.0, 20.0))
            .unwrap();
        sys.add_branch(Branch::new(1, "L12", 1, 2, 0.01, 0.05))
            .unwrap();
        sys.add_generator(
            Generator::new(1, "G1", GeneratorType::Thermal, 200.0, 0.0)
                .at_bus(1)
                .with_voltage_setpoint(1.05),
        )
        .unwrap();
        sys
    }

    #[test]
    fn dc_two_bus() {
        let r = solve(&two_bus()).unwrap();
        assert!(r.converged);
        // DC power flow is lossless; with 1% R, the loss is ~0.5%, so P
        // should be ~50 MW ± a few percent.
        let p = r.branch_flows[0].p_from_mw;
        assert!((p - 50.0).abs() < 2.5, "p_from_mw = {p}");
    }
}
