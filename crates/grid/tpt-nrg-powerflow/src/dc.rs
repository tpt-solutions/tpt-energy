//! DC power flow: linearized active-power-only flow.

use tpt_nrg_core::{BusType, EnergySystem};
use tpt_nrg_topology::AdmittanceMatrixBuilder;

use crate::result::PowerFlowResult;
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

    let y = AdmittanceMatrixBuilder::new(system).build();
    let (p_sched, _) = bus_schedules_pu(system);

    // Build B' matrix (per-unit susceptance, ignoring resistance).
    let n_red = n - 1;
    let mut b_red = vec![0.0_f64; n_red * n_red];
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
            // B'[i,j] = -B[i,j] for off-diagonals; -sum_k B[i,k] for diagonal
            if i == j {
                let mut sum = 0.0;
                for k in 0..n {
                    if k == slack {
                        continue;
                    }
                    // Approximate "removed" element by ignoring it
                    sum += y.b_ij(i, k);
                }
                b_red[row * n_red + col] = -sum;
            } else {
                b_red[row * n_red + col] = -y.b_ij(i, j);
            }
            row += 1;
        }
        col += 1;
    }
    // RHS: p_sched excluding slack
    let mut rhs = Vec::with_capacity(n_red);
    for i in 0..n {
        if i == slack {
            continue;
        }
        rhs.push(p_sched[i]);
    }
    let theta_red = solve_dense(n_red, &b_red, &rhs);
    // Place the angles back
    let mut theta = vec![0.0_f64; n];
    let mut k = 0;
    for i in 0..n {
        if i == slack {
            continue;
        }
        theta[i] = theta_red[k];
        k += 1;
    }
    // V = 1.0 pu (DC power flow)
    let v: Vec<f64> = (0..n).map(|i| match system.buses[i].bus_type {
        BusType::Slack | BusType::Pv => system.buses[i].voltage_magnitude_pu,
        _ => 1.0,
    }).collect();

    // Compute branch flows: P_ij = (θ_i - θ_j) / x_ij (pu)
    let map = bus_index_map(system);
    let base = system.base_mva;
    let mut flows = Vec::with_capacity(system.branches.len());
    let mut total_p = 0.0;
    for br in &system.branches {
        let i = map.get(br.from_bus).and_then(|x| *x);
        let j = map.get(br.to_bus).and_then(|x| *x);
        let (p_from_mw, p_to_mw) = match (i, j) {
            (Some(i), Some(j)) => {
                if br.reactance_pu.abs() < 1e-12 {
                    (0.0, 0.0)
                } else {
                    let p_pu = (theta[i] - theta[j]) / br.reactance_pu;
                    let p_mw = p_pu * base;
                    (p_mw, -p_mw)
                }
            }
            _ => (0.0, 0.0),
        };
        total_p += p_from_mw + p_to_mw;
        let s = p_from_mw.abs();
        let loading = if br.rating_mva > 0.0 { s / br.rating_mva } else { 0.0 };
        flows.push(crate::result::BranchFlow {
            id: br.id,
            p_from_mw: p_from_mw,
            q_from_mvar: 0.0,
            p_to_mw: p_to_mw,
            q_to_mvar: 0.0,
            loading_fraction: loading,
        });
    }
    let total_losses_mw = 0.5 * total_p; // DC ignores losses (sum should be 0)

    let mut gen_p = vec![0.0_f64; system.generators.len()];
    let map = bus_index_map(system);
    for (k, g) in system.generators.iter().enumerate() {
        if !g.in_service {
            continue;
        }
        if let Some(Some(bi)) = map.get(g.bus_id) {
            gen_p[k] = p_sched[*bi] * base;
        }
    }
    let gen_q = vec![0.0_f64; system.generators.len()];

    Ok(PowerFlowResult {
        converged: true,
        iterations: 1,
        final_mismatch: 0.0,
        bus_voltage_magnitude_pu: v,
        bus_voltage_angle_rad: theta,
        branch_flows: flows,
        total_losses_mw,
        total_losses_mvar: 0.0,
        generator_p_mw: gen_p,
        generator_q_mvar: gen_q,
    })
}

fn map_validate(e: tpt_nrg_core::CoreError) -> PowerFlowError {
    match e {
        tpt_nrg_core::CoreError::NoSlackBus => PowerFlowError::NoSlackBus,
        tpt_nrg_core::CoreError::MultipleSlackBuses(n) => PowerFlowError::MultipleSlackBuses(n),
        other => PowerFlowError::Other(other.to_string()),
    }
}

fn solve_dense(n: usize, a_mat: &[f64], b: &[f64]) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    let mut a = a_mat.to_vec();
    let mut x = b.to_vec();
    for k in 0..n {
        let mut max_val = a[k * n + k].abs();
        let mut max_row = k;
        for r in (k + 1)..n {
            if (a[r * n + k]).abs() > max_val {
                max_val = a[r * n + k].abs();
                max_row = r;
            }
        }
        if max_row != k {
            for c in 0..n {
                a.swap(k * n + c, max_row * n + c);
            }
            x.swap(k, max_row);
        }
        let pivot = a[k * n + k];
        if pivot.abs() < 1e-14 {
            continue;
        }
        for r in (k + 1)..n {
            let factor = a[r * n + k] / pivot;
            for c in k..n {
                a[r * n + c] -= factor * a[k * n + c];
            }
            x[r] -= factor * x[k];
        }
    }
    for i in (0..n).rev() {
        let mut sum = x[i];
        for j in (i + 1)..n {
            sum -= a[i * n + j] * x[j];
        }
        let diag = a[i * n + i];
        x[i] = if diag.abs() > 1e-14 { sum / diag } else { 0.0 };
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
        assert!((p - 50.0).abs() < 2.0, "p_from_mw = {p}");
    }
}
