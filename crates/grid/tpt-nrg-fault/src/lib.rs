//! # tpt-nrg-fault
//!
//! Short-circuit (fault) analysis using the symmetrical-component method:
//! three-phase, line-to-line, line-to-ground, and double line-to-ground
//! faults.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use tpt_nrg_core::EnergySystem;
use tpt_nrg_topology::AdmittanceMatrixBuilder;

/// Type of fault to analyse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FaultType {
    /// Symmetric three-phase fault.
    ThreePhase,
    /// Line-to-line fault.
    LineToLine,
    /// Single line-to-ground fault.
    LineToGround,
    /// Double line-to-ground fault.
    DoubleLineToGround,
}

/// Result of a fault calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultResult {
    /// Type of fault that was analysed.
    pub fault_type: FaultType,
    /// Bus id where the fault was applied.
    pub fault_bus: usize,
    /// Positive-sequence fault current magnitude (pu).
    pub i_positive_mag_pu: f64,
    /// Negative-sequence fault current magnitude (pu). Zero for three-phase.
    pub i_negative_mag_pu: f64,
    /// Zero-sequence fault current magnitude (pu).
    pub i_zero_mag_pu: f64,
    /// Per-phase RMS fault current at the faulted bus (pu).
    pub i_fault_pu: f64,
    /// Per-phase RMS fault current at the faulted bus (A on system base).
    pub i_fault_amps: f64,
    /// Positive-sequence Thevenin impedance at the faulted bus (pu).
    pub z1_pu: f64,
    /// Negative-sequence Thevenin impedance at the faulted bus (pu).
    pub z2_pu: f64,
    /// Zero-sequence Thevenin impedance at the faulted bus (pu).
    pub z0_pu: f64,
}

/// Sequence currents (symmetrical components).
#[derive(Debug, Clone, Copy)]
pub struct SequenceCurrents {
    /// Positive-sequence current (pu).
    pub i1: f64,
    /// Negative-sequence current (pu).
    pub i2: f64,
    /// Zero-sequence current (pu).
    pub i0: f64,
}

/// Fault analyzer.
pub struct FaultAnalyzer<'a> {
    system: &'a EnergySystem,
}

impl<'a> FaultAnalyzer<'a> {
    /// Create a new fault analyzer.
    pub fn new(system: &'a EnergySystem) -> Self {
        Self { system }
    }

    /// Compute the positive-sequence Thevenin impedance at the given bus
    /// by inverting the Y-bus and looking at the diagonal entry.
    pub fn z1_thevenin(&self, bus: usize) -> f64 {
        let y = AdmittanceMatrixBuilder::new(self.system).build();
        let n = y.n;
        let i = self.bus_index(bus);
        let zbus = invert_matrix(n, &y.g, &y.b);
        let idx = i * n + i;
        let z_re = zbus.0[idx];
        let z_im = zbus.1[idx];
        (z_re * z_re + z_im * z_im).sqrt()
    }

    /// Thevenin impedance using the direct series-impedance approximation
    /// (no Y-bus inversion). For a radial network this gives the
    /// short-circuit impedance at the fault bus as the sum of series
    /// impedances from the source. Fast and robust for planning studies.
    pub fn z1_thevenin_radial(&self, bus: usize) -> f64 {
        // BFS from the slack bus, accumulating series impedance.
        let slack = self
            .system
            .buses
            .iter()
            .find(|b| b.bus_type == tpt_nrg_core::BusType::Slack)
            .map(|b| b.id);
        let Some(slack) = slack else { return 1.0 };
        if slack == bus {
            // Fault at the slack — assume internal source impedance.
            return 0.001;
        }
        // Dijkstra: cost = path resistance squared + reactance squared
        use std::collections::HashMap;
        let mut dist: HashMap<usize, (f64, f64)> = HashMap::new();
        dist.insert(slack, (0.0, 0.0));
        let mut queue: std::collections::BinaryHeap<(std::cmp::Reverse<u64>, usize)> =
            std::collections::BinaryHeap::new();
        queue.push((std::cmp::Reverse(0), slack));
        while let Some((_, u)) = queue.pop() {
            if u == bus {
                let (r, x) = dist[&bus];
                return (r * r + x * x).sqrt();
            }
            for br in &self.system.branches {
                let (v, _) = if br.from_bus == u {
                    (br.to_bus, false)
                } else if br.to_bus == u {
                    (br.from_bus, true)
                } else {
                    continue;
                };
                if !br.in_service {
                    continue;
                }
                let (r_acc, x_acc) = dist[&u];
                let (r_new, x_new) = (r_acc + br.resistance_pu, x_acc + br.reactance_pu);
                let cur = dist.get(&v).copied();
                let better = match cur {
                    Some((r, x)) => r_new * r_new + x_new * x_new < r * r + x * x,
                    None => true,
                };
                if better {
                    dist.insert(v, (r_new, x_new));
                    let mag = ((r_new * r_new + x_new * x_new) * 1e6) as u64;
                    queue.push((std::cmp::Reverse(mag), v));
                }
            }
        }
        1.0
    }

    /// Simplified fault-current calculation at the given bus.
    ///
    /// Uses the radial-network approximation (series-impedance sum) for
    /// `Z₁`. Assumes `Z₂ = Z₁` and `Z₀ = 3·Z₁` (typical for systems with
    /// neutral grounding reactors).
    pub fn calculate_fault_current(
        &self,
        bus: usize,
        fault_type: FaultType,
    ) -> FaultResult {
        let z1 = self.z1_thevenin_radial(bus);
        let z2 = z1; // assumption
        let z0 = 3.0 * z1; // assumption; real systems vary
        let pre_voltage = 1.0; // pre-fault voltage in pu (flat)

        let seq = match fault_type {
            FaultType::ThreePhase => SequenceCurrents {
                i1: pre_voltage / z1,
                i2: 0.0,
                i0: 0.0,
            },
            FaultType::LineToLine => {
                let i1 = pre_voltage / (z1 + z2);
                SequenceCurrents { i1, i2: -i1, i0: 0.0 }
            }
            FaultType::LineToGround => {
                let i1 = pre_voltage / (z1 + z2 + z0);
                SequenceCurrents { i1, i2: i1, i0: i1 }
            }
            FaultType::DoubleLineToGround => {
                let z_sum = (z2 * z0) / (z2 + z0);
                let i1 = pre_voltage / (z1 + z_sum);
                SequenceCurrents {
                    i1,
                    i2: -i1 * z0 / (z2 + z0),
                    i0: -i1 * z2 / (z2 + z0),
                }
            }
        };

        let i_fault_pu = match fault_type {
            FaultType::ThreePhase => seq.i1.abs(),
            FaultType::LineToLine => ((3.0_f64).sqrt() * seq.i1.abs()),
            FaultType::LineToGround => (3.0 * seq.i1.abs()),
            FaultType::DoubleLineToGround => {
                let a = (1.0 + 2.0 * seq.i2 / seq.i1).abs();
                let b = (seq.i0 / seq.i1).abs();
                3.0 * seq.i0.abs()
            }
        };

        let base_amps = self.system.base_mva * 1.0e6
            / ((self.bus_base_kv(bus) * 1000.0) * (3.0_f64).sqrt());
        let i_fault_amps = i_fault_pu * base_amps;

        FaultResult {
            fault_type,
            fault_bus: bus,
            i_positive_mag_pu: seq.i1.abs(),
            i_negative_mag_pu: seq.i2.abs(),
            i_zero_mag_pu: seq.i0.abs(),
            i_fault_pu,
            i_fault_amps,
            z1_pu: z1,
            z2_pu: z2,
            z0_pu: z0,
        }
    }

    fn bus_index(&self, bus: usize) -> usize {
        self.system
            .buses
            .iter()
            .position(|b| b.id == bus)
            .unwrap_or(0)
    }

    fn bus_base_kv(&self, bus: usize) -> f64 {
        self.system
            .buses
            .iter()
            .find(|b| b.id == bus)
            .map(|b| b.base_kv)
            .unwrap_or(110.0)
    }
}

/// Invert a dense complex matrix `(G + jB)` using real arithmetic. Returns
/// `(re_z, im_z)` in row-major form.
fn invert_matrix(n: usize, g: &[f64], b: &[f64]) -> (Vec<f64>, Vec<f64>) {
    // Build a 2n × 2n real matrix.
    let mut a = vec![0.0_f64; 4 * n * n];
    // Top-left: G, Top-right: -B
    // Bottom-left: B, Bottom-right: G
    for i in 0..n {
        for j in 0..n {
            a[i * 2 * n + j] = g[i * n + j];
            a[i * 2 * n + (n + j)] = -b[i * n + j];
            a[(n + i) * 2 * n + j] = b[i * n + j];
            a[(n + i) * 2 * n + (n + j)] = g[i * n + j];
        }
    }
    // Augment with identity
    let mut aug = vec![0.0_f64; 2 * n * 4 * n];
    for i in 0..2 * n {
        for j in 0..2 * n {
            aug[i * 4 * n + j] = a[i * 2 * n + j];
        }
        aug[i * 4 * n + (2 * n + i)] = 1.0;
    }
    // Gaussian elimination
    for k in 0..2 * n {
        let mut max_val = aug[k * 4 * n + k].abs();
        let mut max_row = k;
        for r in (k + 1)..2 * n {
            if (aug[r * 4 * n + k]).abs() > max_val {
                max_val = (aug[r * 4 * n + k]).abs();
                max_row = r;
            }
        }
        if max_row != k {
            for c in 0..4 * n {
                aug.swap(k * 4 * n + c, max_row * 4 * n + c);
            }
        }
        let pivot = aug[k * 4 * n + k];
        if pivot.abs() < 1e-14 {
            continue;
        }
        for r in 0..2 * n {
            if r == k {
                continue;
            }
            let factor = aug[r * 4 * n + k] / pivot;
            if factor == 0.0 {
                continue;
            }
            for c in 0..4 * n {
                aug[r * 4 * n + c] -= factor * aug[k * 4 * n + c];
            }
        }
        for c in 0..4 * n {
            aug[k * 4 * n + c] /= pivot;
        }
    }
    let mut z_re = vec![0.0_f64; n * n];
    let mut z_im = vec![0.0_f64; n * n];
    for i in 0..n {
        for j in 0..n {
            z_re[i * n + j] = aug[i * 4 * n + (2 * n + j)];
            z_im[i * n + j] = aug[(n + i) * 4 * n + (2 * n + j)];
        }
    }
    (z_re, z_im)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_nrg_core::{Branch, Bus, BusType, Generator, GeneratorType};

    fn small() -> EnergySystem {
        let mut sys = EnergySystem::new("s", "S", 100.0, 60.0);
        sys.add_bus(Bus::new(1, "B1", BusType::Slack).with_voltage_pu(1.0, 0.0))
            .unwrap();
        sys.add_bus(Bus::new(2, "B2", BusType::Pq).with_base_kv(110.0))
            .unwrap();
        sys.add_branch(Branch::new(1, "L12", 1, 2, 0.01, 0.10))
            .unwrap();
        sys.add_generator(
            Generator::new(1, "G1", GeneratorType::Thermal, 100.0, 0.0).at_bus(1),
        )
        .unwrap();
        sys
    }

    #[test]
    fn three_phase_fault_at_bus_2() {
        let binding = small();
        let a = FaultAnalyzer::new(&binding);
        let r = a.calculate_fault_current(2, FaultType::ThreePhase);
        // Z1 ≈ |Z_branch| = 0.10 pu (dominated by reactance for 0.01, 0.10)
        // I = 1.0 / 0.10 = 10 pu
        assert!(r.i_fault_pu > 5.0, "i_fault = {}", r.i_fault_pu);
        // I_amps = 10 * 100e6 / (110e3 * sqrt(3)) ≈ 5248 A
        assert!(r.i_fault_amps > 4000.0, "amps = {}", r.i_fault_amps);
    }

    #[test]
    fn slg_fault_less_than_3ph() {
        let binding = small();
        let a = FaultAnalyzer::new(&binding);
        let r3 = a.calculate_fault_current(2, FaultType::ThreePhase);
        let r1lg = a.calculate_fault_current(2, FaultType::LineToGround);
        // SLG current is 3*V/(Z1+Z2+Z0) = 3/(5·Z1) = 0.6/Z1
        // Three-phase is V/Z1 = 1/Z1. So SLG < 3ph when Z0 > 2·Z1.
        assert!(r1lg.i_fault_pu < r3.i_fault_pu);
    }
}
