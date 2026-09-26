//! Newton–Raphson AC power flow solver with generator Q-limit enforcement.
//!
//! The solver supports PV → PQ bus-type switching: when a generator bus's
//! reactive output would exceed its `q_max`/`q_min`, the bus is converted
//! to a PQ bus with the reactive injection fixed at the violated limit;
//! it is converted back once the solved voltage moves back on the far
//! side of the setpoint. This is the standard Dommel–Tinney formulation
//! and is what lets ill-conditioned cases such as IEEE 57-bus converge to
//! their published solution.

use log::debug;
use tpt_nrg_core::{BusType, EnergySystem};
use tpt_nrg_topology::{AdmittanceMatrix, AdmittanceMatrixBuilder};

use crate::result::PowerFlowResult;
use crate::solver::{PowerFlowError, PowerFlowOptions};
use crate::util::{
    bus_index_map, bus_loads_mw, bus_schedules_pu, compute_branch_flows, find_slack,
    flat_start_voltages, net_injection_pu,
};

/// Reactive-violation threshold (p.u.) before a PV bus is converted to PQ.
const Q_VIOLATION_PU: f64 = 1.0e-4;
/// Mismatch (p.u.) below which the reactive limits are evaluated: the
/// outer Q-limit loop runs only on a near-converged inner iterate.
const Q_LIMIT_EVAL_MISMATCH_PU: f64 = 1.0e-2;
/// Voltage margin (p.u.) required beyond the setpoint before a limited
/// bus is released back to PV (prevents chattering).
const V_RELEASE_MARGIN_PU: f64 = 1.0e-3;

/// Damping limits for the Newton step, tuned so ill-conditioned systems
/// (IEEE 57/118) avoid limit cycles while well-conditioned systems
/// (IEEE 14, 30) still converge in the standard 3–7 iterations.
const MAX_ANGLE_STEP_RAD: f64 = 0.2;
const MAX_V_STEP_PU: f64 = 0.05;

/// Immutable per-iteration problem data: admittance entries and the
/// active-power schedule. The reactive schedule is passed per iteration
/// because PV → PQ switching rewrites it at limited buses.
struct NrProblem {
    n: usize,
    g: Vec<f64>,
    b: Vec<f64>,
    p_sched: Vec<f64>,
}

impl NrProblem {
    /// Bus power injections and the resulting mismatches.
    ///
    /// Returns `(dp, dq, p_inj, q_inj)` where the injection vectors are the
    /// raw computed values (before any PV-bus mismatch zeroing).
    fn injections_and_mismatches(
        &self,
        v: &[f64],
        theta: &[f64],
        q_sched: &[f64],
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let n = self.n;
        let mut dp = vec![0.0_f64; n];
        let mut dq = vec![0.0_f64; n];
        let mut p_inj = vec![0.0_f64; n];
        let mut q_inj = vec![0.0_f64; n];
        for i in 0..n {
            let mut p = 0.0;
            let mut q = 0.0;
            let vi = v[i];
            let ti = theta[i];
            for k in 0..n {
                let dt = ti - theta[k];
                let gik = self.g[i * n + k];
                let bik = self.b[i * n + k];
                p += v[k] * (gik * dt.cos() + bik * dt.sin());
                q += v[k] * (gik * dt.sin() - bik * dt.cos());
            }
            p_inj[i] = p * vi;
            q_inj[i] = q * vi;
            dp[i] = self.p_sched[i] - p_inj[i];
            dq[i] = q_sched[i] - q_inj[i];
        }
        (dp, dq, p_inj, q_inj)
    }

    /// Build the reduced Jacobian for the given bus classification.
    ///
    /// `angle_idx` are the non-slack buses (P-θ equations); `v_idx` are the
    /// buses whose voltage magnitude is a variable (PQ buses and PV buses
    /// currently limited at a Q constraint). Returns the row-major
    /// `n_eq × n_eq` Jacobian and the RHS (mismatch) vector.
    fn build_jacobian(
        &self,
        v: &[f64],
        theta: &[f64],
        q_sched: &[f64],
        p_inj: &[f64],
        q_inj: &[f64],
        angle_idx: &[usize],
        v_idx: &[usize],
    ) -> (Vec<f64>, Vec<f64>) {
        let n = self.n;
        let n_theta = angle_idx.len();
        let n_eq = n_theta + v_idx.len();
        let mut jac = vec![0.0_f64; n_eq * n_eq];
        let mut rhs = vec![0.0_f64; n_eq];

        for (row, &i) in angle_idx.iter().enumerate() {
            rhs[row] = self.p_sched[i] - p_inj[i];
        }
        for (row, &i) in v_idx.iter().enumerate() {
            rhs[n_theta + row] = q_sched[i] - q_inj[i];
        }

        // dP/dθ block.
        for (row, &i) in angle_idx.iter().enumerate() {
            for (col, &k) in angle_idx.iter().enumerate() {
                let dt = theta[i] - theta[k];
                let entry = if i == k {
                    -q_inj[i] - self.b[i * n + i] * v[i] * v[i]
                } else {
                    v[i] * v[k] * (self.g[i * n + k] * dt.sin() - self.b[i * n + k] * dt.cos())
                };
                jac[row * n_eq + col] = entry;
            }
        }
        // dP/d|V| block.
        for (row, &i) in angle_idx.iter().enumerate() {
            for (col, &k) in v_idx.iter().enumerate() {
                let dt = theta[i] - theta[k];
                let entry = if i == k {
                    p_inj[i] / v[i] + self.g[i * n + i] * v[i]
                } else {
                    v[i] * (self.g[i * n + k] * dt.cos() + self.b[i * n + k] * dt.sin())
                };
                jac[row * n_eq + n_theta + col] = entry;
            }
        }
        // dQ/dθ block.
        for (row, &i) in v_idx.iter().enumerate() {
            for (col, &k) in angle_idx.iter().enumerate() {
                let dt = theta[i] - theta[k];
                let entry = if i == k {
                    p_inj[i] - self.g[i * n + i] * v[i] * v[i]
                } else {
                    -v[i] * v[k] * (self.g[i * n + k] * dt.cos() + self.b[i * n + k] * dt.sin())
                };
                jac[(n_theta + row) * n_eq + col] = entry;
            }
        }
        // dQ/d|V| block.
        for (row, &i) in v_idx.iter().enumerate() {
            for (col, &k) in v_idx.iter().enumerate() {
                let dt = theta[i] - theta[k];
                let entry = if i == k {
                    q_inj[i] / v[i] - self.b[i * n + i] * v[i]
                } else {
                    v[i] * (self.g[i * n + k] * dt.sin() - self.b[i * n + k] * dt.cos())
                };
                jac[(n_theta + row) * n_eq + n_theta + col] = entry;
            }
        }
        (jac, rhs)
    }
}

/// Aggregate per-bus generator reactive limits in p.u.
///
/// Buses without an in-service generator report `(−INF, +INF)`.
fn generator_q_limits_pu(system: &EnergySystem) -> (Vec<f64>, Vec<f64>) {
    let n = system.buses.len();
    let mut q_min = vec![f64::INFINITY; n];
    let mut q_max = vec![f64::NEG_INFINITY; n];
    let map = bus_index_map(system);
    let base = system.base_mva;
    for gen in &system.generators {
        if !gen.in_service {
            continue;
        }
        if let Some(Some(idx)) = map.get(gen.bus_id) {
            q_min[*idx] = q_min[*idx].min(gen.q_min_mvar / base);
            q_max[*idx] = q_max[*idx].max(gen.q_max_mvar / base);
        }
    }
    // Buses without an in-service generator get inert (−INF, +INF) limits
    // so the violation checks can never fire there.
    for i in 0..n {
        if q_max[i] == f64::NEG_INFINITY {
            q_min[i] = f64::NEG_INFINITY;
            q_max[i] = f64::INFINITY;
        }
    }
    (q_min, q_max)
}

/// Solve the AC power flow using the Newton–Raphson method with PV → PQ
/// switching at generator reactive-power limits.
///
/// # Errors
///
/// Returns [`PowerFlowError`] when the system fails validation or the
/// solver fails to converge within `options.max_iterations`.
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
    let (p_sched, mut q_sched) = bus_schedules_pu(system);
    let (mut v, mut theta) = warm_start_voltages(system);

    let problem = NrProblem {
        n,
        g: y_bus.g.clone(),
        b: y_bus.b.clone(),
        p_sched,
    };
    let (q_min_pu, q_max_pu) = generator_q_limits_pu(system);
    let base = system.base_mva;
    let load_q_pu: Vec<f64> = bus_loads_mw(system).1.iter().map(|&q| q / base).collect();
    let v_setpoint: Vec<f64> = system
        .buses
        .iter()
        .map(|b| b.voltage_magnitude_pu)
        .collect();

    // Q-limit state: a bus currently enforced at its reactive limit.
    let mut limited_at_max = vec![false; n];
    let mut limited_at_min = vec![false; n];

    let mut iterations = 0;
    let mut final_mismatch = f64::INFINITY;
    for it in 0..options.max_iterations {
        iterations = it + 1;

        // Dynamic bus classification: PV buses sitting at a Q limit are
        // solved as PQ buses (their voltage becomes a variable).
        let angle_idx: Vec<usize> = (0..n).filter(|&i| i != slack).collect();
        let v_idx: Vec<usize> = (0..n)
            .filter(|&i| {
                i != slack
                    && (system.buses[i].bus_type == BusType::Pq
                        || limited_at_max[i]
                        || limited_at_min[i])
            })
            .collect();

        let (dp, mut dq, p_inj, q_inj) = problem.injections_and_mismatches(&v, &theta, &q_sched);
        // Only report the Q mismatch at buses whose Q equation is enforced.
        for &i in &angle_idx {
            let q_enforced =
                system.buses[i].bus_type == BusType::Pq || limited_at_max[i] || limited_at_min[i];
            if !q_enforced {
                dq[i] = 0.0;
            }
        }

        // Convergence check on the enforced mismatches.
        let max_mis = enforced_mismatch(&dp, &dq, &angle_idx, &v_idx, iterations)?;

        // PV → PQ switching at reactive limits, and release back to PV.
        // Limits are only evaluated once the iteration is close to
        // converged (the classic outer-loop approach): switching on
        // transient mid-iteration injections over-constrains the solve.
        let switched = if max_mis < Q_LIMIT_EVAL_MISMATCH_PU {
            enforce_q_limits(
                system,
                slack,
                &q_min_pu,
                &q_max_pu,
                &q_inj,
                &v,
                &load_q_pu,
                &v_setpoint,
                &mut limited_at_max,
                &mut limited_at_min,
                &mut q_sched,
            )
        } else {
            false
        };
        final_mismatch = max_mis;
        debug!("NR iter {it}: mismatch = {max_mis:.3e}");
        if std::env::var("TPT_NR_TRACE").is_ok() {
            trace_progress(it, max_mis, &dp, &dq);
        }
        // Converged only when no limit state changed this iteration: a
        // switch changes the enforced equations, so at least one more
        // iteration is required.
        if max_mis < options.tolerance && !switched {
            break;
        }

        newton_step(
            &problem, &q_sched, &p_inj, &q_inj, &angle_idx, &v_idx, &mut v, &mut theta,
        );
    }

    let converged = final_mismatch < options.tolerance;
    if !converged {
        return Err(PowerFlowError::NonConvergence {
            iterations,
            mismatch: final_mismatch,
        });
    }

    // Restore voltage setpoints only for buses still solving as PV; buses
    // limited at a Q constraint keep their solved PQ voltage.
    restore_pv_setpoints(system, &mut v, &limited_at_max, &limited_at_min);

    Ok(finalize_result(
        system,
        slack,
        &y_bus,
        &problem,
        &q_sched,
        v,
        theta,
        iterations,
        final_mismatch,
        converged,
    ))
}

/// Apply one round of PV ↔ PQ limit switching.
///
/// A PV bus whose computed reactive injection exceeds `q_max` (or falls
/// below `q_min`) is converted to a PQ bus with its reactive schedule
/// pinned at the violated limit; a limited bus whose solved voltage has
/// moved back across the setpoint is released.
#[allow(clippy::too_many_arguments)]
fn enforce_q_limits(
    system: &EnergySystem,
    slack: usize,
    q_min_pu: &[f64],
    q_max_pu: &[f64],
    q_inj: &[f64],
    v: &[f64],
    load_q_pu: &[f64],
    v_setpoint: &[f64],
    limited_at_max: &mut [bool],
    limited_at_min: &mut [bool],
    q_sched: &mut [f64],
) -> bool {
    let mut switched = false;
    for (i, bus) in system.buses.iter().enumerate() {
        if i == slack || !matches!(bus.bus_type, BusType::Pv) {
            continue;
        }
        if limited_at_max[i] {
            // Released once holding the setpoint needs ≤ q_max again.
            if v[i] > v_setpoint[i] + V_RELEASE_MARGIN_PU {
                limited_at_max[i] = false;
                switched = true;
            }
        } else if limited_at_min[i] {
            if v[i] < v_setpoint[i] - V_RELEASE_MARGIN_PU {
                limited_at_min[i] = false;
                switched = true;
            }
        } else {
            // The limits constrain the *generator's* reactive output, which
            // is the net bus injection plus the bus load (loads are not in
            // the Y-bus injection; shunts already are).
            let q_gen = q_inj[i] + load_q_pu[i];
            if q_gen > q_max_pu[i] + Q_VIOLATION_PU {
                limited_at_max[i] = true;
                switched = true;
                q_sched[i] = q_max_pu[i] - load_q_pu[i];
            } else if q_gen < q_min_pu[i] - Q_VIOLATION_PU {
                limited_at_min[i] = true;
                switched = true;
                q_sched[i] = q_min_pu[i] - load_q_pu[i];
            }
        }
    }
    switched
}

/// Take one damped Newton step: build and solve the reduced Jacobian
/// system, then update the angle and voltage vectors with per-iteration
/// step limits.
fn newton_step(
    problem: &NrProblem,
    q_sched: &[f64],
    p_inj: &[f64],
    q_inj: &[f64],
    angle_idx: &[usize],
    v_idx: &[usize],
    v: &mut [f64],
    theta: &mut [f64],
) {
    let (jac, rhs) = problem.build_jacobian(v, theta, q_sched, p_inj, q_inj, angle_idx, v_idx);
    let n_theta = angle_idx.len();
    let dx = solve_dense(angle_idx.len() + v_idx.len(), &jac, &rhs);
    for (idx, &i) in angle_idx.iter().enumerate() {
        let step = dx[idx].clamp(-MAX_ANGLE_STEP_RAD, MAX_ANGLE_STEP_RAD);
        theta[i] += step;
    }
    for (idx, &i) in v_idx.iter().enumerate() {
        let step = dx[n_theta + idx].clamp(-MAX_V_STEP_PU, MAX_V_STEP_PU);
        v[i] = (v[i] + step).clamp(0.5, 1.5);
    }
}

/// Restore the scheduled voltage magnitude at buses still solving as PV.
fn restore_pv_setpoints(
    system: &EnergySystem,
    v: &mut [f64],
    limited_at_max: &[bool],
    limited_at_min: &[bool],
) {
    for (i, bus) in system.buses.iter().enumerate() {
        if matches!(bus.bus_type, BusType::Pv) && !limited_at_max[i] && !limited_at_min[i] {
            v[i] = bus.voltage_magnitude_pu;
        }
    }
}

/// Maximum absolute enforced mismatch (P at non-slack buses, Q at buses
/// whose Q equation is enforced), rejecting non-finite values.
fn enforced_mismatch(
    dp: &[f64],
    dq: &[f64],
    angle_idx: &[usize],
    v_idx: &[usize],
    iterations: usize,
) -> Result<f64, PowerFlowError> {
    let mut max_mis: f64 = 0.0;
    for &i in angle_idx {
        max_mis = check_finite(dp[i], iterations, max_mis)?;
    }
    for &i in v_idx {
        max_mis = check_finite(dq[i], iterations, max_mis)?;
    }
    Ok(max_mis)
}

/// Guard one mismatch value: `NaN`/`∞` means divergence, not slow progress.
fn check_finite(value: f64, iterations: usize, current_max: f64) -> Result<f64, PowerFlowError> {
    if !value.is_finite() {
        // NaN/∞ means divergence, not slow progress; f64::max would
        // silently swallow it and report false convergence.
        return Err(PowerFlowError::NonConvergence {
            iterations,
            mismatch: value,
        });
    }
    Ok(current_max.max(value.abs()))
}

fn trace_progress(it: usize, max_mis: f64, dp: &[f64], dq: &[f64]) {
    let worst = dp
        .iter()
        .enumerate()
        .max_by(|a, b| {
            a.1.abs()
                .partial_cmp(&b.1.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map_or((0, 0.0), |(i, val)| (i, *val));
    let worst_q = dq
        .iter()
        .enumerate()
        .max_by(|a, b| {
            a.1.abs()
                .partial_cmp(&b.1.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map_or((0, 0.0), |(i, val)| (i, *val));
    eprintln!(
        "NR iter {it}: mismatch = {max_mis:.6e} (worst P: bus {} = {:+.3e}, worst Q: bus {} = {:+.3e})",
        worst.0, worst.1, worst_q.0, worst_q.1
    );
}

/// DC warm start followed by the JSON-driven initial voltages.
fn warm_start_voltages(system: &EnergySystem) -> (Vec<f64>, Vec<f64>) {
    let (v, mut theta) = flat_start_voltages(system);
    // DC power-flow warm start: solve `B'·θ = P_sched` for the non-slack
    // angles. This dramatically improves convergence for systems with
    // widely varying voltage angles (e.g. IEEE 57/118).
    if let Ok(dc) = crate::dc::solve(system) {
        for (i, t) in theta.iter_mut().enumerate() {
            if i < dc.bus_voltage_angle_rad.len() {
                *t = dc.bus_voltage_angle_rad[i];
            }
        }
    }
    (v, theta)
}

/// Assemble the final [`PowerFlowResult`] from the solved state.
fn finalize_result(
    system: &EnergySystem,
    slack: usize,
    y_bus: &AdmittanceMatrix,
    problem: &NrProblem,
    q_sched: &[f64],
    v: Vec<f64>,
    theta: Vec<f64>,
    iterations: usize,
    final_mismatch: f64,
    converged: bool,
) -> PowerFlowResult {
    let flows = compute_branch_flows(system, y_bus, &v, &theta);
    // Each branch's p_from + p_to is its series loss (shunt B is lossless),
    // so the sum over all branches is the total system loss.
    let total_losses_mw: f64 = flows.iter().map(|f| f.p_from_mw + f.p_to_mw).sum();
    let total_losses_mvar: f64 = flows.iter().map(|f| f.q_from_mvar + f.q_to_mvar).sum();

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
            // non-slack buses the scheduled injection is exact at
            // convergence; at the slack bus the solved injection is used
            // because the solver ignores the slack schedule.
            let (p_inj, q_inj) = net_injection_pu(y_bus, &v, &theta, *bi);
            let p_bus_pu = if *bi == slack {
                p_inj
            } else {
                problem.p_sched[*bi]
            };
            gen_p[k] = p_bus_pu * system.base_mva + load_p[*bi];
            gen_q[k] = q_inj * system.base_mva + load_q[*bi];
        }
    }
    let _ = q_sched;

    PowerFlowResult {
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
    }
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
    use crate::solver::{PowerFlowMethod, PowerFlowSolver};
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
        assert!(r.converged);
        // Slack voltage should remain at 1.05
        assert!((r.bus_voltage_magnitude_pu[0] - 1.05).abs() < 1e-6);
        // Receiving-end voltage should drop slightly due to losses
        assert!(r.bus_voltage_magnitude_pu[1] < 1.05);
        // Lossless-ish: P from slack ≈ 50 MW
        assert!((r.branch_flows[0].p_from_mw - 50.0).abs() < 1.0);
    }

    #[test]
    fn q_limit_switching_binds_and_converges() {
        // A PV generator with a tiny q_max cannot hold its setpoint against
        // a heavy reactive load: the bus must switch to PQ with Q pinned at
        // the limit, the voltage must sag below the setpoint, and the
        // solver must still converge.
        let mut sys = EnergySystem::new("qlim", "Q-limit", 100.0, 60.0);
        sys.add_bus(Bus::new(1, "Slack", BusType::Slack).with_voltage_pu(1.0, 0.0))
            .unwrap();
        sys.add_bus(
            Bus::new(2, "PV", BusType::Pv)
                .with_voltage_pu(1.0, 0.0)
                .with_load(40.0, 40.0),
        )
        .unwrap();
        sys.add_branch(Branch::new(1, "L12", 1, 2, 0.01, 0.1))
            .unwrap();
        let mut gen = Generator::new(1, "G1", GeneratorType::Thermal, 100.0, 0.0).at_bus(2);
        gen.voltage_setpoint_pu = 1.0;
        gen.q_max_mvar = 5.0;
        gen.q_min_mvar = -5.0;
        sys.add_generator(gen).unwrap();

        let solver = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson)
            .with_tolerance(1e-8)
            .with_max_iterations(100);
        let r = solver.solve(&sys).unwrap();
        assert!(r.converged, "must converge with the limit binding");
        // Reactive output is pinned at the limit (±1 kVAr slack).
        assert!(
            (r.generator_q_mvar[0] - 5.0).abs() < 1e-3,
            "gen Q = {} should sit at q_max",
            r.generator_q_mvar[0]
        );
        // The bus can no longer hold its 1.0 pu setpoint.
        assert!(
            r.bus_voltage_magnitude_pu[1] < 0.99,
            "V = {} must sag below setpoint",
            r.bus_voltage_magnitude_pu[1]
        );
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
        assert!(
            r.converged,
            "did not converge: mismatch={}",
            r.final_mismatch
        );
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
