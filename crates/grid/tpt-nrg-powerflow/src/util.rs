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
    // Add explicit generators
    for g in &system.generators {
        if !g.in_service {
            continue;
        }
        if let Some(Some(idx)) = map.get(g.bus_id) {
            p_sched[*idx] += g.p_schedule_mw / base;
        }
    }
    (p_sched, q_sched)
}

/// Initialize the voltage vector (flat start, 1.0 pu, 0 rad).
pub(crate) fn flat_start_voltages(system: &EnergySystem) -> (Vec<f64>, Vec<f64>) {
    let n = system.buses.len();
    let mut v = vec![1.0_f64; n];
    let mut theta = vec![0.0_f64; n];
    for (i, b) in system.buses.iter().enumerate() {
        v[i] = b.voltage_magnitude_pu;
        theta[i] = b.voltage_angle_rad;
    }
    (v, theta)
}

/// Compute branch flows for the solved voltage profile.
pub(crate) fn compute_branch_flows(
    system: &EnergySystem,
    y: &AdmittanceMatrix,
    v: &[f64],
    theta: &[f64],
) -> Vec<crate::result::BranchFlow> {
    let map = bus_index_map(system);
    let base = system.base_mva;
    let mut flows = Vec::with_capacity(system.branches.len());
    for br in &system.branches {
        let i = match map.get(br.from_bus).and_then(|x| *x) {
            Some(v) => v,
            None => {
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
        };
        let j = match map.get(br.to_bus).and_then(|x| *x) {
            Some(v) => v,
            None => {
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
        };
        let vi = v[i];
        let vj = v[j];
        let ti = theta[i];
        let tj = theta[j];

        // Use the off-diagonal Y entry to get the line admittance.
        // For a simple line, y_ij = -y_series (and y_ji is the conjugate for
        // symmetric lines). For off-nominal taps the off-diagonals are
        // -y/t or -y/t*, but for line flows we use the standard formula:
        //   S_from = V_i * (Y_ii V_i + Y_ij V_j)^*,  S_to = V_j * (Y_ji V_i + Y_jj V_j)^*.
        let g_ii = y.g_ij(i, i);
        let b_ii = y.b_ij(i, i);
        let g_ij = y.g_ij(i, j);
        let b_ij = y.b_ij(i, j);
        let g_ji = y.g_ij(j, i);
        let b_ji = y.b_ij(j, i);
        let g_jj = y.g_ij(j, j);
        let b_jj = y.b_ij(j, j);

        // I_i = (G_ii + jB_ii) V_i e^{jθ_i} + (G_ij + jB_ij) V_j e^{jθ_j}
        let (sx_re, sx_im) = mul(
            g_ii,
            b_ii,
            vi * ti.cos(),
            vi * ti.sin(),
        );
        let (sx_re2, sx_im2) = mul(g_ij, b_ij, vj * tj.cos(), vj * tj.sin());
        let i_from_re = sx_re + sx_re2;
        let i_from_im = sx_im + sx_im2;
        // S_from = V_i e^{jθ_i} * (I_from)^*  -> take conjugate
        let v_from_re = vi * ti.cos();
        let v_from_im = vi * ti.sin();
        let s_from_re = v_from_re * i_from_re + v_from_im * i_from_im;
        let s_from_im = -v_from_re * i_from_im + v_from_im * i_from_re;

        // I_j similarly
        let (sy_re, sy_im) = mul(g_ji, b_ji, vi * ti.cos(), vi * ti.sin());
        let (sy_re2, sy_im2) = mul(g_jj, b_jj, vj * tj.cos(), vj * tj.sin());
        let i_to_re = sy_re + sy_re2;
        let i_to_im = sy_im + sy_im2;
        let v_to_re = vj * tj.cos();
        let v_to_im = vj * tj.sin();
        let s_to_re = v_to_re * i_to_re + v_to_im * i_to_im;
        let s_to_im = -v_to_re * i_to_im + v_to_im * i_to_re;

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

/// Multiply complex numbers: `(a+jb) * (c+jd) = (ac - bd) + j(ad + bc)`.
fn mul(a: f64, b: f64, c: f64, d: f64) -> (f64, f64) {
    (a * c - b * d, a * d + b * c)
}
