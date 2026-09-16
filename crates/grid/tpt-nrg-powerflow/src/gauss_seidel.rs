//! Gauss–Seidel AC power flow solver.

use tpt_nrg_core::{BusType, EnergySystem};
use tpt_nrg_topology::AdmittanceMatrixBuilder;

use crate::result::PowerFlowResult;
use crate::solver::{PowerFlowError, PowerFlowOptions};
use crate::util::{
    bus_index_map, bus_loads_mw, bus_schedules_pu, compute_branch_flows, find_slack,
    flat_start_voltages, net_injection_pu,
};

/// Solve the AC power flow using the Gauss–Seidel method with optional
/// acceleration.
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
    let (p_sched, q_sched) = bus_schedules_pu(system);
    let (mut v, mut theta) = flat_start_voltages(system);

    let accel = options.acceleration;
    let mut iterations = 0;
    let mut final_mismatch = f64::INFINITY;
    for it in 0..options.max_iterations {
        iterations = it + 1;
        let mut max_mis: f64 = 0.0;
        for i in 0..n {
            if i == slack {
                continue;
            }
            let bus = &system.buses[i];
            // V_i_new = (1/Y_ii) * ((P_i - jQ_i)/V_i* - sum_{k!=i} Y_ik V_k)
            let mut sum_re = 0.0;
            let mut sum_im = 0.0;
            for k in 0..n {
                if k == i {
                    continue;
                }
                let gik = y_bus.g_ij(i, k);
                let bik = y_bus.b_ij(i, k);
                let vk_re = v[k] * theta[k].cos();
                let vk_im = v[k] * theta[k].sin();
                sum_re += gik * vk_re - bik * vk_im;
                sum_im += gik * vk_im + bik * vk_re;
            }
            let gii = y_bus.g_ij(i, i);
            let bii = y_bus.b_ij(i, i);
            // (P - jQ) / V* = (P - jQ) * V / |V|^2
            let p = p_sched[i];
            let q = q_sched[i];
            let vmag2 = v[i] * v[i];
            let num_re = (p * v[i] * theta[i].cos() + q * v[i] * theta[i].sin()) / vmag2;
            let num_im = (p * v[i] * theta[i].sin() - q * v[i] * theta[i].cos()) / vmag2;
            // v_new = (num - sum) / (g + jb)
            let rhs_re = num_re - sum_re;
            let rhs_im = num_im - sum_im;
            let denom = gii * gii + bii * bii;
            let vnew_re = if denom > 1e-20 {
                (rhs_re * gii + rhs_im * bii) / denom
            } else {
                v[i] * theta[i].cos()
            };
            let vnew_im = if denom > 1e-20 {
                (rhs_im * gii - rhs_re * bii) / denom
            } else {
                v[i] * theta[i].sin()
            };
            let mut vnew = (vnew_re * vnew_re + vnew_im * vnew_im).sqrt();
            let mut thetanew = vnew_im.atan2(vnew_re);
            // Apply acceleration
            v[i] = v[i] + accel * (vnew - v[i]);
            theta[i] = theta[i] + accel * (thetanew - theta[i]);
            // For PV buses, snap V to the setpoint
            if matches!(bus.bus_type, BusType::Pv) {
                v[i] = bus.voltage_magnitude_pu;
            }
            // Compute mismatch for convergence check
            // P_inj = V_i * sum_k V_k (G cos + B sin)
            let mut p_inj = 0.0;
            let mut q_inj = 0.0;
            for k in 0..n {
                let dt = theta[i] - theta[k];
                p_inj += v[k] * (y_bus.g_ij(i, k) * dt.cos() + y_bus.b_ij(i, k) * dt.sin());
                q_inj += v[k] * (y_bus.g_ij(i, k) * dt.sin() - y_bus.b_ij(i, k) * dt.cos());
            }
            p_inj *= v[i];
            q_inj *= v[i];
            let dp = (p_sched[i] - p_inj).abs();
            let dq = if matches!(bus.bus_type, BusType::Pv) {
                0.0
            } else {
                (q_sched[i] - q_inj).abs()
            };
            // A non-finite mismatch means the iteration diverged; f64::max
            // would silently swallow NaN and report false convergence.
            if !dp.is_finite() || !dq.is_finite() {
                return Err(PowerFlowError::NonConvergence {
                    iterations,
                    mismatch: f64::NAN,
                });
            }
            max_mis = max_mis.max(dp).max(dq);
        }
        final_mismatch = max_mis;
        if max_mis < options.tolerance {
            break;
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
    for (k, g) in system.generators.iter().enumerate() {
        if !g.in_service {
            continue;
        }
        if let Some(Some(bi)) = map.get(g.bus_id) {
            // Generator output = solved net bus injection + bus load; at the
            // slack bus the injection is the solved value.
            let (p_inj, q_inj) = net_injection_pu(&y_bus, &v, &theta, *bi);
            let p_bus_pu = if *bi == slack { p_inj } else { p_sched[*bi] };
            gen_p[k] = p_bus_pu * system.base_mva + load_p[*bi];
            gen_q[k] = q_inj * system.base_mva + load_q[*bi];
        }
    }

    let converged = final_mismatch < options.tolerance;
    if !converged {
        return Err(PowerFlowError::NonConvergence {
            iterations,
            mismatch: final_mismatch,
        });
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
    fn gs_two_bus_converges() {
        let solver_opts = PowerFlowOptions {
            tolerance: 1e-6,
            max_iterations: 5000,
            acceleration: 1.2,
        };
        let r = solve(&two_bus(), solver_opts).unwrap();
        assert!(r.converged);
        assert!((r.bus_voltage_magnitude_pu[0] - 1.05).abs() < 1e-6);
    }
}
