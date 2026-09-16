//! Fast Decoupled Power Flow (FDPF, Stott & Alsac 1974, BX version).
//!
//! The FDPF is a simplification of Newton–Raphson that uses two decoupled
//! constant Jacobian matrices: one for P–θ and one for Q–V. Each iteration
//! alternates between solving two much smaller linear systems, avoiding
//! the cost of refactoring the full Jacobian at every step.
//!
//! This implementation uses the **XB** variant (i.e. the B' matrix uses the
//! negative of the imaginary part of Y-bus, while B'' uses the imaginary
//! part without shunt contributions).

use tpt_nrg_core::{BusType, EnergySystem};
use tpt_nrg_topology::AdmittanceMatrixBuilder;

use crate::result::PowerFlowResult;
use crate::solver::{PowerFlowError, PowerFlowOptions};
use crate::util::{
    bus_index_map, bus_loads_mw, bus_schedules_pu, compute_branch_flows, find_slack,
    flat_start_voltages, net_injection_pu,
};

/// Solve the AC power flow using the Fast Decoupled (XB) method.
#[allow(clippy::too_many_lines)]
pub fn solve(
    system: &EnergySystem,
    options: PowerFlowOptions,
) -> Result<PowerFlowResult, PowerFlowError> {
    system.validate().map_err(map_validate)?;
    let n = system.buses.len();
    if n == 0 {
        return Err(PowerFlowError::NoSlackBus);
    }
    let slack = find_slack(system)?;
    let y_bus = AdmittanceMatrixBuilder::new(system).build();
    let g = &y_bus.g;
    let b = &y_bus.b;
    let (p_sched, q_sched) = bus_schedules_pu(system);
    let (mut v, mut theta) = flat_start_voltages(system);

    // Build B' (P-θ) — uses -Im(Y) including shunts.
    // Build B'' (Q-V) — uses -Im(Y) without shunts.
    let mut bp = vec![0.0_f64; n * n];
    let mut bpp = vec![0.0_f64; n * n];
    for i in 0..n {
        for j in 0..n {
            let val_p = -b[i * n + j];
            let mut val_q = -b[i * n + j];
            if i == j {
                val_q += system.buses[i].shunt_susceptance_pu;
            }
            bp[i * n + j] = val_p;
            bpp[i * n + j] = val_q;
        }
    }

    // Non-slack index lists.
    let n_theta: Vec<usize> = (0..n).filter(|&i| i != slack).collect();
    let n_v: Vec<usize> = (0..n)
        .filter(|&i| i != slack && system.buses[i].bus_type == BusType::Pq)
        .collect();

    // Reduced matrices.
    let mut bp_red = vec![0.0_f64; n_theta.len() * n_theta.len()];
    let mut bpp_red = vec![0.0_f64; n_v.len() * n_v.len()];
    for (ri, &i) in n_theta.iter().enumerate() {
        for (rj, &j) in n_theta.iter().enumerate() {
            bp_red[ri * n_theta.len() + rj] = bp[i * n + j];
        }
    }
    for (ri, &i) in n_v.iter().enumerate() {
        for (rj, &j) in n_v.iter().enumerate() {
            bpp_red[ri * n_v.len() + rj] = bpp[i * n + j];
        }
    }

    let mut iterations = 0_usize;
    let mut final_mismatch = f64::INFINITY;
    for it in 0..options.max_iterations {
        iterations = it + 1;
        // Compute mismatches.
        let mut dp = vec![0.0_f64; n];
        let mut dq = vec![0.0_f64; n];
        for i in 0..n {
            let vi = v[i];
            let ti = theta[i];
            let mut p_inj = 0.0;
            let mut q_inj = 0.0;
            for k in 0..n {
                let vk = v[k];
                let dt = ti - theta[k];
                let gik = g[i * n + k];
                let bik = b[i * n + k];
                p_inj += vk * (gik * dt.cos() + bik * dt.sin());
                q_inj += vk * (gik * dt.sin() - bik * dt.cos());
            }
            p_inj *= vi;
            q_inj *= vi;
            dp[i] = p_sched[i] - p_inj;
            dq[i] = q_sched[i] - q_inj;
        }
        for (i, bus) in system.buses.iter().enumerate() {
            if i == slack {
                continue;
            }
            if matches!(bus.bus_type, BusType::Pv) {
                dq[i] = 0.0;
            }
        }

        let mut max_mis: f64 = 0.0;
        for &i in &n_theta {
            let a = dp[i].abs();
            if !a.is_finite() {
                // f64::max silently swallows NaN — bail out on divergence.
                return Err(PowerFlowError::NonConvergence {
                    iterations,
                    mismatch: a,
                });
            }
            max_mis = max_mis.max(a);
        }
        for &i in &n_v {
            let a = dq[i].abs();
            if !a.is_finite() {
                return Err(PowerFlowError::NonConvergence {
                    iterations,
                    mismatch: a,
                });
            }
            max_mis = max_mis.max(a);
        }
        final_mismatch = max_mis;
        if max_mis < options.tolerance {
            break;
        }

        // Solve Δθ = B'⁻¹ · ΔP and ΔV = B''⁻¹ · ΔQ
        let mut rhs_t = Vec::with_capacity(n_theta.len());
        for &i in &n_theta {
            rhs_t.push(dp[i]);
        }
        let dtheta = solve_dense(n_theta.len(), &bp_red, &rhs_t);

        let mut rhs_v = Vec::with_capacity(n_v.len());
        for &i in &n_v {
            rhs_v.push(dq[i] / v[i].max(0.5));
        }
        let dv_div = solve_dense(n_v.len(), &bpp_red, &rhs_v);

        // Apply updates with FDPF acceleration factor (default 1.0).
        for (idx, &i) in n_theta.iter().enumerate() {
            theta[i] += dtheta[idx];
        }
        for (idx, &i) in n_v.iter().enumerate() {
            v[i] = (v[i] + dv_div[idx] * v[i].max(0.5)).clamp(0.5, 1.5);
        }
    }

    let converged = final_mismatch < options.tolerance;
    if !converged {
        return Err(PowerFlowError::NonConvergence {
            iterations,
            mismatch: final_mismatch,
        });
    }

    for (i, bus) in system.buses.iter().enumerate() {
        if matches!(bus.bus_type, BusType::Pv) {
            v[i] = bus.voltage_magnitude_pu;
        }
    }

    let flows = compute_branch_flows(system, &y_bus, &v, &theta);
    // Each branch's p_from + p_to is its series loss (shunt B is lossless),
    // so the sum over all branches is the total system loss.
    let mut total_p = 0.0;
    let mut total_q = 0.0;
    for f in &flows {
        total_p += f.p_from_mw + f.p_to_mw;
        total_q += f.q_from_mvar + f.q_to_mvar;
    }
    let total_losses_mw = total_p;
    let total_losses_mvar = total_q;

    let mut gen_p = vec![0.0_f64; system.generators.len()];
    let mut gen_q = vec![0.0_f64; system.generators.len()];
    let map = bus_index_map(system);
    let (load_p, load_q) = bus_loads_mw(system);
    for (k, gen) in system.generators.iter().enumerate() {
        if !gen.in_service {
            continue;
        }
        if let Some(Some(bi)) = map.get(gen.bus_id) {
            // Generator output = solved net bus injection + bus load; at the
            // slack bus the injection is the solved value.
            let (p_inj, q_inj) = net_injection_pu(&y_bus, &v, &theta, *bi);
            let p_bus_pu = if *bi == slack { p_inj } else { p_sched[*bi] };
            gen_p[k] = p_bus_pu * system.base_mva + load_p[*bi];
            gen_q[k] = q_inj * system.base_mva + load_q[*bi];
        }
    }

    Ok(PowerFlowResult {
        converged,
        iterations,
        final_mismatch,
        bus_voltage_magnitude_pu: v,
        bus_voltage_angle_rad: theta,
        branch_flows: flows,
        total_losses_mw,
        total_losses_mvar,
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
        if max_val < 1e-14 {
            continue;
        }
        if max_row != k {
            for c in 0..n {
                a.swap(k * n + c, max_row * n + c);
            }
            x.swap(k, max_row);
        }
        let pivot = a[k * n + k];
        for r in (k + 1)..n {
            let factor = a[r * n + k] / pivot;
            if factor == 0.0 {
                continue;
            }
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
    fn fdpf_two_bus_converges() {
        let sys = two_bus();
        let r = solve(
            &sys,
            PowerFlowOptions {
                tolerance: 1e-6,
                max_iterations: 100,
                acceleration: 1.0,
            },
        )
        .unwrap();
        assert!(r.converged, "mismatch = {}", r.final_mismatch);
        assert!((r.bus_voltage_magnitude_pu[0] - 1.05).abs() < 1e-6);
        assert!(r.bus_voltage_magnitude_pu[1] < 1.05);
    }
}
