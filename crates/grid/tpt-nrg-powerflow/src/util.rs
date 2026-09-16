//! Shared helper routines for power-flow solvers.

use tpt_nrg_core::{Bus, BusType, EnergySystem};
use tpt_nrg_topology::AdmittanceMatrix;

use crate::solver::PowerFlowError;

/// Map `bus.id` to a dense 0-based index.
pub(crate) fn bus_index_map(system: &EnergySystem) -> Vec<Option<usize>> {
    let max_id = system.buses.iter().map(|b| b.id).max().unwrap_or(0);
    let mut idx = vec![None; max_id + 1];
    for (i, b) in system.buses.iter().enumerate() {
        if b.id <= max_id {
            idx[b.id] = Some(i);
        }
    }
    idx
}

/// Find the slack bus index (dense 0-based).
pub(crate) fn find_slack(system: &EnergySystem) -> Result<usize, PowerFlowError> {
    let slacks: Vec<&Bus> = system
        .buses
        .iter()
        .filter(|b| b.bus_type == BusType::Slack)
        .collect();
    if slacks.is_empty() {
        return Err(PowerFlowError::NoSlackBus);
    }
    if slacks.len() > 1 {
        return Err(PowerFlowError::MultipleSlackBuses(slacks.len()));
    }
    // Find its dense index
    let id = slacks[0].id;
    system
        .buses
        .iter()
        .position(|b| b.id == id)
        .ok_or(PowerFlowError::UnknownBusId(id))
}

/// Compute the scheduled P/Q (in p.u.) at every bus: positive = generation,
/// positive = load consumption. PV buses have `p_sched` and `v_sched`; PQ buses
/// have `p_sched = -load`.
pub(crate) fn bus_schedules_pu(system: &EnergySystem) -> (Vec<f64>, Vec<f64>) {
    let base = system.base_mva;
    let n = system.buses.len();
    let mut p_sched = vec![0.0_f64; n];
    let mut q_sched = vec![0.0_f64; n];
    for (i, b) in system.buses.iter().enumerate() {
        p_sched[i] = (b.generation_mw - b.load_mw) / base;
        q_sched[i] = (b.generation_mvar - b.load_mvar) / base;
    }
    // Add explicit loads on the same dense index
    let map = bus_index_map(system);
    for l in &system.loads {
        if !l.in_service {
            continue;
        }
        if let Some(Some(idx)) = map.get(l.bus_id) {
            p_sched[*idx] -= l.p_mw / base;
            q_sched[*idx] -= l.q_mvar / base;
        }
    }
    // Add explicit generators (P only — Q is free for PV/slack buses).
    for g in &system.generators {
        if !g.in_service {
            continue;
        }
        if let Some(Some(idx)) = map.get(g.bus_id) {
            p_sched[*idx] += g.p_schedule_mw / base;
            // Zero out Q schedule at generator buses — Q is the unknown for PV
            // buses, and slack Q is determined by the system balance.
            if matches!(system.buses[*idx].bus_type, tpt_nrg_core::BusType::Pv | tpt_nrg_core::BusType::Slack) {
                q_sched[*idx] = 0.0;
            }
        }
    }
    (p_sched, q_sched)
}

/// Per-bus load in MW / MVAr, combining bus-level and standalone loads.
pub(crate) fn bus_loads_mw(system: &EnergySystem) -> (Vec<f64>, Vec<f64>) {
    let n = system.buses.len();
    let mut p = vec![0.0_f64; n];
    let mut q = vec![0.0_f64; n];
    for (i, b) in system.buses.iter().enumerate() {
        p[i] = b.load_mw;
        q[i] = b.load_mvar;
    }
    let map = bus_index_map(system);
    for l in &system.loads {
        if !l.in_service {
            continue;
        }
        if let Some(Some(idx)) = map.get(l.bus_id) {
            p[*idx] += l.p_mw;
            q[*idx] += l.q_mvar;
        }
    }
    (p, q)
}

/// Solved net complex power injection (p.u.) at dense bus index `i`.
pub(crate) fn net_injection_pu(
    y_bus: &AdmittanceMatrix,
    v: &[f64],
    theta: &[f64],
    i: usize,
) -> (f64, f64) {
    let n = y_bus.n;
    let mut p_inj = 0.0;
    let mut q_inj = 0.0;
    for k in 0..n {
        let dt = theta[i] - theta[k];
        let gik = y_bus.g_ij(i, k);
        let bik = y_bus.b_ij(i, k);
        p_inj += v[k] * (gik * dt.cos() + bik * dt.sin());
        q_inj += v[k] * (gik * dt.sin() - bik * dt.cos());
    }
    (p_inj * v[i], q_inj * v[i])
}

/// Initialize the voltage vector.
///
/// Strategy:
/// 1. If any bus in the JSON has a non-trivial `voltage_angle_rad` schedule
///    (i.e. not all zero), use those angles AND voltage magnitudes as the
///    warm start — this lets cases like IEEE 57-bus, where the published
///    profile spans ±0.5 rad in angle and ±10% in |V|, start much closer
///    to the solution. PQ buses normally have no setpoint, but if the
///    JSON includes one it is treated as an *initial guess* — the solver
///    is still free to move it.
/// 2. Otherwise fall back to a flat start (1.0 pu, 0 rad) on PQ buses, with
///    PV/slack buses at their voltage-magnitude setpoint and angle = 0.
pub(crate) fn flat_start_voltages(system: &EnergySystem) -> (Vec<f64>, Vec<f64>) {
    let n = system.buses.len();
    let mut v = vec![1.0_f64; n];
    let mut theta = vec![0.0_f64; n];

    // Detect "JSON-supplied angle schedule" vs "default flat start".
    let supplied = system
        .buses
        .iter()
        .any(|b| b.voltage_angle_rad.abs() > 1e-6);

    for (i, b) in system.buses.iter().enumerate() {
        v[i] = match b.bus_type {
            tpt_nrg_core::BusType::Slack | tpt_nrg_core::BusType::Pv => b.voltage_magnitude_pu,
            _ => 1.0,
        };
        theta[i] = if supplied { b.voltage_angle_rad } else { 0.0 };
    }
    (v, theta)
}

/// Compute branch flows for the solved voltage profile.
///
/// The flow at the "from" end of a branch is computed from the branch's
/// series admittance and the **transformed** voltage at each end:
///
/// ```text
/// I_from = (V_from / t) · y_series · e^{-jθ} − V_from · j·(B/2)
/// S_from = V_from · I_from^*
/// ```
///
/// where `t = tap_ratio * e^{j·phase_shift}` is the complex turns ratio.
/// This formulation is valid for both transmission lines (`t = 1`) and
/// off-nominal transformers.
pub(crate) fn compute_branch_flows(
    system: &EnergySystem,
    _y: &AdmittanceMatrix,
    v: &[f64],
    theta: &[f64],
) -> Vec<crate::result::BranchFlow> {
    let map = bus_index_map(system);
    let base = system.base_mva;
    let mut flows = Vec::with_capacity(system.branches.len());
    for br in &system.branches {
        let i = match map.get(br.from_bus).and_then(|x| *x) {
            Some(v) => v,
            None => continue,
        };
        let j = match map.get(br.to_bus).and_then(|x| *x) {
            Some(v) => v,
            None => continue,
        };
        let r = br.resistance_pu;
        let x = br.reactance_pu;
        let denom = r * r + x * x;
        if denom == 0.0 {
            flows.push(crate::result::BranchFlow {
                id: br.id,
                p_from_mw: 0.0,
                q_from_mvar: 0.0,
                p_to_mw: 0.0,
                q_to_mvar: 0.0,
                loading_fraction: 0.0,
            });
            continue;
        }
        let g_series = r / denom;
        let b_series = -x / denom;
        // B/2 at each end (split line charging)
        let bc_half = br.susceptance_pu * 0.5;
        let tap = br.tap_ratio;
        let shift = br.phase_shift_rad;

        let vi = v[i];
        let ti = theta[i];
        let vj = v[j];
        let tj = theta[j];

        // V_from_internal = V_i / t (t complex: t = tap * e^{j·shift})
        // 1 / (tap e^{j shift}) = (1/tap) e^{-j shift}
        let v_int_mag = vi / tap;
        let v_int_ang = ti - shift;
        let v_int_re = v_int_mag * v_int_ang.cos();
        let v_int_im = v_int_mag * v_int_ang.sin();

        // V_to = V_j e^{j θ_j}
        let v_to_re = vj * tj.cos();
        let v_to_im = vj * tj.sin();

        // I_from_internal = y_series · (V_from_internal − V_to)
        let dv_re = v_int_re - v_to_re;
        let dv_im = v_int_im - v_to_im;
        // (g + j b)(dv_re + j dv_im) = (g·dv_re - b·dv_im) + j(g·dv_im + b·dv_re)
        let i_int_re = g_series * dv_re - b_series * dv_im;
        let i_int_im = g_series * dv_im + b_series * dv_re;
        // I_from (on the "from" bus side) = I_from_internal / t*
        //   = I_int · e^{+j shift} / tap
        let i_from_re = (i_int_re * shift.cos() - i_int_im * shift.sin()) / tap;
        let i_from_im = (i_int_re * shift.sin() + i_int_im * shift.cos()) / tap;
        // Shunt current at "from" end: j·(B/2) · V_i
        let ish_re = -bc_half * vi * ti.sin();
        let ish_im = bc_half * vi * ti.cos();
        let i_from_tot_re = i_from_re + ish_re;
        let i_from_tot_im = i_from_im + ish_im;
        // S_from = V_i · I_from*
        let v_from_re = vi * ti.cos();
        let v_from_im = vi * ti.sin();
        let s_from_re = v_from_re * i_from_tot_re + v_from_im * i_from_tot_im;
        let s_from_im = -v_from_re * i_from_tot_im + v_from_im * i_from_tot_re;

        // I_to_internal = -y_series · (V_from_internal − V_to) = -I_int
        // I_to (on the "to" bus side) = I_to_internal
        let i_to_re = -i_int_re;
        let i_to_im = -i_int_im;
        // Shunt current at "to" end: j·(B/2) · V_j
        let ish2_re = -bc_half * vj * tj.sin();
        let ish2_im = bc_half * vj * tj.cos();
        let i_to_tot_re = i_to_re + ish2_re;
        let i_to_tot_im = i_to_im + ish2_im;
        let v_to_re_full = v_to_re;
        let v_to_im_full = v_to_im;
        let s_to_re = v_to_re_full * i_to_tot_re + v_to_im_full * i_to_tot_im;
        let s_to_im = -v_to_re_full * i_to_tot_im + v_to_im_full * i_to_tot_re;

        let p_from = s_from_re * base;
        let q_from = s_from_im * base;
        let p_to = s_to_re * base;
        let q_to = s_to_im * base;

        let s_from_mag = (p_from * p_from + q_from * q_from).sqrt();
        let loading = if br.rating_mva > 0.0 {
            s_from_mag / br.rating_mva
        } else {
            0.0
        };

        flows.push(crate::result::BranchFlow {
            id: br.id,
            p_from_mw: p_from,
            q_from_mvar: q_from,
            p_to_mw: p_to,
            q_to_mvar: q_to,
            loading_fraction: loading,
        });
    }
    flows
}
