//! Newton–Raphson AC power flow solver.

use log::debug;
use tpt_nrg_core::{BusType, EnergySystem};
use tpt_nrg_topology::AdmittanceMatrixBuilder;

use crate::result::PowerFlowResult;
use crate::solver::{PowerFlowError, PowerFlowMethod, PowerFlowOptions, PowerFlowSolver};
use crate::util::{
    bus_index_map, bus_loads_mw, bus_schedules_pu, compute_branch_flows, find_slack,
    flat_start_voltages, net_injection_pu,
};

/// Solve the AC power flow using the Newton–Raphson method.
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

    // DC power-flow warm start: solve `B'·θ = P_sched` for the non-slack
    // angles and use those as initial conditions for AC Newton–Raphson. This
    // dramatically improves convergence for systems with widely varying
    // voltage angles (e.g. IEEE 57/118).
    if let Ok(dc) = crate::dc::solve(system) {
        for (i, t) in theta.iter_mut().enumerate() {
            if i < dc.bus_voltage_angle_rad.len() {
                *t = dc.bus_voltage_angle_rad[i];
            }
        }
    }

    // Variable ordering: angle of every non-slack bus, then |V| of every
    // non-slack PQ bus (PV buses have V fixed at the setpoint).
    let mut angle_idx: Vec<usize> = (0..n).filter(|&i| i != slack).collect();
    let mut v_idx: Vec<usize> = (0..n)
        .filter(|&i| i != slack && system.buses[i].bus_type == BusType::Pq)
        .collect();
    let n_theta = angle_idx.len();
    let n_v = v_idx.len();
    let n_eq = n_theta + n_v;

    let mut iterations = 0;
    let mut final_mismatch = f64::INFINITY;
    for it in 0..options.max_iterations {
        iterations = it + 1;

        // Mismatches ΔP, ΔQ at all non-slack buses.
        let mut dp = vec![0.0_f64; n];
        let mut dq = vec![0.0_f64; n];
        for i in 0..n {
            let mut p_inj = 0.0;
            let mut q_inj = 0.0;
            let vi = v[i];
            let ti = theta[i];
            for k in 0..n {
                let vk = v[k];
                let tk = theta[k];
                let dt = ti - tk;
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
        // Only report P mismatch at PV buses; Q mismatch is irrelevant there.
        for (i, bus) in system.buses.iter().enumerate() {
            if i == slack {
                continue;
            }
            if matches!(bus.bus_type, BusType::Pv) {
                dq[i] = 0.0;
            }
        }

        // Build the reduced Jacobian
        let mut jac = vec![0.0_f64; n_eq * n_eq];
        let mut rhs = vec![0.0_f64; n_eq];

        // Helper closures for indexing
        let col_of_theta = |idx: usize| idx;
        let col_of_v = |idx: usize| n_theta + idx;

        for (ii, &i) in angle_idx.iter().enumerate() {
            rhs[ii] = dp[i];
        }
        for (ii, &i) in v_idx.iter().enumerate() {
            rhs[n_theta + ii] = dq[i];
        }

        // Fill dP/dθ block
        for (ii, &i) in angle_idx.iter().enumerate() {
            for (kk, &k) in angle_idx.iter().enumerate() {
                let vk = v[k];
                let dt = theta[i] - theta[k];
                let gik = g[i * n + k];
                let bik = b[i * n + k];
                if i == k {
                    // ∂P_i/∂θ_i = -Q_i - B_ii V_i²
                    let vi = v[i];
                    let q_inj = {
                        let mut s = 0.0;
                        for kk2 in 0..n {
                            let dt2 = theta[i] - theta[kk2];
                            s += v[kk2]
                                * (g[i * n + kk2] * dt2.sin() - b[i * n + kk2] * dt2.cos());
                        }
                        vi * s
                    };
                    jac[ii * n_eq + col_of_theta(kk)] = -q_inj - b[i * n + i] * vi * vi;
                } else {
                    // ∂P_i/∂θ_k = V_i V_k (G_ik sin θ_ik - B_ik cos θ_ik)
                    let vi = v[i];
                    jac[ii * n_eq + col_of_theta(kk)] =
                        vi * vk * (gik * dt.sin() - bik * dt.cos());
                }
            }
        }
        // Fill dP/d|V| block
        for (ii, &i) in angle_idx.iter().enumerate() {
            for (kk, &k) in v_idx.iter().enumerate() {
                let dt = theta[i] - theta[k];
                let gik = g[i * n + k];
                let bik = b[i * n + k];
                if i == k {
                    // ∂P_i/∂V_i = P_i / V_i + G_ii V_i
                    let vi = v[i];
                    let p_inj = {
                        let mut s = 0.0;
                        for kk2 in 0..n {
                            let dt2 = theta[i] - theta[kk2];
                            s += v[kk2]
                                * (g[i * n + kk2] * dt2.cos() + b[i * n + kk2] * dt2.sin());
                        }
                        vi * s
                    };
                    jac[ii * n_eq + col_of_v(kk)] = p_inj / vi + g[i * n + i] * vi;
                } else {
                    // ∂P_i/∂V_k = V_i (G_ik cos θ_ik + B_ik sin θ_ik)
                    jac[ii * n_eq + col_of_v(kk)] = v[i] * (gik * dt.cos() + bik * dt.sin());
                }
            }
        }
        // Fill dQ/dθ block
        for (ii, &i) in v_idx.iter().enumerate() {
            for (kk, &k) in angle_idx.iter().enumerate() {
                let vk = v[k];
                let dt = theta[i] - theta[k];
                let gik = g[i * n + k];
                let bik = b[i * n + k];
                if i == k {
                    let vi = v[i];
                    let p_inj = {
                        let mut s = 0.0;
                        for kk2 in 0..n {
                            let dt2 = theta[i] - theta[kk2];
                            s += v[kk2]
                                * (g[i * n + kk2] * dt2.cos() + b[i * n + kk2] * dt2.sin());
                        }
                        vi * s
                    };
                    jac[(n_theta + ii) * n_eq + col_of_theta(kk)] = p_inj - g[i * n + i] * vi * vi;
                } else {
                    // ∂Q_i/∂θ_k = -V_i V_k (G_ik cos θ_ik + B_ik sin θ_ik)
                    jac[(n_theta + ii) * n_eq + col_of_theta(kk)] =
                        -v[i] * vk * (gik * dt.cos() + bik * dt.sin());
                }
            }
        }
        // Fill dQ/d|V| block
        for (ii, &i) in v_idx.iter().enumerate() {
            for (kk, &k) in v_idx.iter().enumerate() {
                let dt = theta[i] - theta[k];
                let gik = g[i * n + k];
                let bik = b[i * n + k];
                if i == k {
                    // ∂Q_i/∂V_i = Q_i / V_i - B_ii V_i
                    let vi = v[i];
                    let q_inj = {
                        let mut s = 0.0;
                        for kk2 in 0..n {
                            let dt2 = theta[i] - theta[kk2];
                            s += v[kk2]
                                * (g[i * n + kk2] * dt2.sin() - b[i * n + kk2] * dt2.cos());
                        }
                        vi * s
                    };
                    jac[(n_theta + ii) * n_eq + col_of_v(kk)] = q_inj / vi - b[i * n + i] * vi;
                } else {
                    // ∂Q_i/∂V_k = V_i (G_ik sin θ_ik - B_ik cos θ_ik)
                    jac[(n_theta + ii) * n_eq + col_of_v(kk)] =
                        v[i] * (gik * dt.sin() - bik * dt.cos());
                }
            }
        }

        // Convergence check on the relevant mismatches
        let mut max_mis: f64 = 0.0;
        for &i in &angle_idx {
            let a = dp[i].abs();
            if !a.is_finite() {
                // f64::max silently swallows NaN, which would report false
                // convergence on a diverged iteration.
                return Err(PowerFlowError::NonConvergence {
                    iterations,
                    mismatch: a,
                });
            }
            max_mis = max_mis.max(a);
        }
        for &i in &v_idx {
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
        debug!("NR iter {it}: mismatch = {max_mis:.3e}");
        // Trace progress when running with the `trace` feature or when
        // `TPT_NR_TRACE=1` is set in the environment.
        if std::env::var("TPT_NR_TRACE").is_ok() {
            let worst = dp
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
                .map(|(i, v)| (i, *v))
                .unwrap_or((0, 0.0));
            let worst_q = dq
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
                .map(|(i, v)| (i, *v))
                .unwrap_or((0, 0.0));
            eprintln!(
                "NR iter {it}: mismatch = {max_mis:.6e} (worst P: bus {} = {:+.3e}, worst Q: bus {} = {:+.3e})",
                worst.0, worst.1, worst_q.0, worst_q.1
            );
        }
        if max_mis < options.tolerance {
            break;
        }

        let dx = solve_dense(n_eq, &jac, &rhs);
        // Damped Newton–Raphson: limit the per-iteration change in angles
        // and voltages. Aggressive damping (0.2 rad / 0.05 pu) helps ill-
        // conditioned systems like IEEE 57-bus avoid limit cycles while
        // still letting well-conditioned systems (IEEE 14, 30) converge in
        // the standard 3–7 iterations.
        let max_angle_step = 0.2;   // ~11.5° per iteration
        let max_v_step     = 0.05;  // 5% of rated per iteration
        for (idx, &i) in angle_idx.iter().enumerate() {
            let step = dx[idx].clamp(-max_angle_step, max_angle_step);
            theta[i] += step;
        }
        for (idx, &i) in v_idx.iter().enumerate() {
            let step = dx[n_theta + idx].clamp(-max_v_step, max_v_step);
            let new_v = (v[i] + step).clamp(0.5, 1.5);
            v[i] = new_v;
        }
    }

    let converged = final_mismatch < options.tolerance;
    if !converged {
        return Err(PowerFlowError::NonConvergence {
            iterations,
            mismatch: final_mismatch,
        });
    }

    // Restore PV setpoints (the solver should hold them implicitly, but be safe)
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
            // Generator output = solved net bus injection + bus load. At
            // non-slack buses the scheduled injection is exact; at the slack
            // bus the injection is the solved value (the schedule there is
            // ignored by the solver).
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

impl From<tpt_nrg_core::CoreError> for PowerFlowError {
    fn from(e: tpt_nrg_core::CoreError) -> Self {
        map_validate(e)
    }
}

/// Solve a dense linear system `A·x = b` using Gaussian elimination with
/// partial pivoting. `a` is row-major `n*n`, `b` has length `n`.
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
    fn two_bus_converges() {
        let sys = two_bus();
        let solver = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson)
            .with_tolerance(1e-6)
            .with_max_iterations(50);
        let r = solver.solve(&sys).unwrap();
        eprintln!("V = {:?}", r.bus_voltage_magnitude_pu);
        eprintln!("θ = {:?}", r.bus_voltage_angle_rad);
        eprintln!("flows = {:?}", r.branch_flows);
        eprintln!("losses = {} MW", r.total_losses_mw);
        assert!(r.converged);
        // Slack voltage should remain at 1.05
        assert!((r.bus_voltage_magnitude_pu[0] - 1.05).abs() < 1e-6);
        // Receiving-end voltage should drop slightly due to losses
        assert!(r.bus_voltage_magnitude_pu[1] < 1.05);
        // Lossless-ish: P from slack ≈ 50 MW
        assert!((r.branch_flows[0].p_from_mw - 50.0).abs() < 1.0);
    }

    #[test]
    fn ieee14_solves() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("test-data")
            .join("ieee")
            .join("ieee14.json");
        let sys = EnergySystem::from_json_file(&path).expect("load ieee14");
        let solver = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson)
            .with_tolerance(1e-5)
            .with_max_iterations(50);
        let r = solver.solve(&sys).expect("solve ieee14");
        assert!(r.converged, "did not converge: mismatch={}", r.final_mismatch);
        // Slack V at bus 1 ≈ 1.06 pu
        assert!((r.bus_voltage_magnitude_pu[0] - 1.06).abs() < 1e-3);
        // Reference voltage magnitudes for IEEE 14: all in 0.90 - 1.10 pu
        for v in &r.bus_voltage_magnitude_pu {
            assert!(*v > 0.90 && *v < 1.10, "voltage out of band: {v}");
        }
        // Total losses should be small but positive (3-30 MW depending on case).
        assert!(
            r.total_losses_mw > 0.5 && r.total_losses_mw < 30.0,
            "losses = {} MW",
            r.total_losses_mw
        );
        // Per-branch loadings should be reasonable.
        for f in &r.branch_flows {
            assert!(
                f.loading_fraction < 2.0,
                "branch {} overloaded: loading = {}",
                f.id,
                f.loading_fraction
            );
        }
    }
}
