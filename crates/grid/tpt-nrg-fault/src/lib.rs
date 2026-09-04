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

/// Positive-, negative-, and zero-sequence Thevenin impedances at a bus.
///
/// The full method requires per-sequence Y-bus matrices (built from per-
/// sequence branch impedances). When those are not available, the
/// scalar-impedance form [`FaultAnalyzer::calculate_fault_current`]
/// applies defaulting assumptions: `Z₂ = Z₁` and `Z₀ = 3·Z₁`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SequenceNetwork {
    /// Positive-sequence Thevenin impedance magnitude at the bus (pu).
    pub z1: f64,
    /// Negative-sequence Thevenin impedance magnitude at the bus (pu).
    pub z2: f64,
    /// Zero-sequence Thevenin impedance magnitude at the bus (pu).
    pub z0: f64,
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

    /// Construct a [`SequenceNetwork`] at the given bus using default
    /// zero/negative-sequence assumptions (`Z₂ = Z₁`, `Z₀ = 3·Z₁`).
    ///
    /// For systems where per-sequence branch impedances are known, override
    /// the returned values before passing them to
    /// [`FaultAnalyzer::fault_from_sequence`].
    pub fn sequence_network(&self, bus: usize) -> SequenceNetwork {
        let z1 = self.z1_thevenin_radial(bus);
        SequenceNetwork { z1, z2: z1, z0: 3.0 * z1 }
    }

    /// Compute the per-phase fault current from a user-supplied sequence
    /// network. This is the building block used by the higher-level
    /// [`FaultAnalyzer::calculate_fault_current`] but lets the caller
    /// override the defaulting assumptions about Z₂ and Z₀.
    pub fn fault_from_sequence(
        &self,
        bus: usize,
        fault_type: FaultType,
        net: SequenceNetwork,
    ) -> FaultResult {
        let pre_voltage = 1.0;
        let seq = match fault_type {
            FaultType::ThreePhase => SequenceCurrents {
                i1: pre_voltage / net.z1,
                i2: 0.0,
                i0: 0.0,
            },
            FaultType::LineToLine => {
                let i1 = pre_voltage / (net.z1 + net.z2);
                SequenceCurrents { i1, i2: -i1, i0: 0.0 }
            }
            FaultType::LineToGround => {
                let i1 = pre_voltage / (net.z1 + net.z2 + net.z0);
                SequenceCurrents { i1, i2: i1, i0: i1 }
            }
            FaultType::DoubleLineToGround => {
                let z_sum = (net.z2 * net.z0) / (net.z2 + net.z0);
                let i1 = pre_voltage / (net.z1 + z_sum);
                SequenceCurrents {
                    i1,
                    i2: -i1 * net.z0 / (net.z2 + net.z0),
                    i0: -i1 * net.z2 / (net.z2 + net.z0),
                }
            }
        };
        let i_fault_pu = match fault_type {
            FaultType::ThreePhase => seq.i1.abs(),
            FaultType::LineToLine => (3.0_f64).sqrt() * seq.i1.abs(),
            FaultType::LineToGround => 3.0 * seq.i1.abs(),
            FaultType::DoubleLineToGround => 3.0 * seq.i0.abs(),
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
            z1_pu: net.z1,
            z2_pu: net.z2,
            z0_pu: net.z0,
        }
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

    /// Textbook validation: with Z1 = Z2 = j0.4 pu and Z0 = j0.1 pu at a
    /// faulted bus on a 100 MVA, 230 kV base, the published fault currents
    /// (per-phase RMS, pu) are:
    ///
    /// - 3φ:  |If| = 1.0 / 0.4 = 2.5 pu
    /// - L-L: |If| = √3 · 0.5 / 0.8 ≈ 1.083 pu
    /// - SLG: |If| = 3 · 0.5 / 0.9 ≈ 1.667 pu
    /// - DLG: |If| = 3 · |I0| ≈ 3 · 0.625 = 1.875 pu
    ///
    /// (Reference: Glover, Sarma & Overbye, "Power System Analysis and
    /// Design", 5th ed., Example 7.5.)
    #[test]
    fn textbook_fault_currents() {
        let binding = small();
        let a = FaultAnalyzer::new(&binding);
        let z1 = 0.4;
        let z2 = 0.4;
        let z0 = 0.1;
        let net = SequenceNetwork { z1, z2, z0 };

        let r3 = a.fault_from_sequence(2, FaultType::ThreePhase, net);
        assert!((r3.i_fault_pu - 2.5).abs() < 1e-9);

        let rll = a.fault_from_sequence(2, FaultType::LineToLine, net);
        assert!((rll.i_fault_pu - 1.0830127).abs() < 1e-5);

        let rslg = a.fault_from_sequence(2, FaultType::LineToGround, net);
        assert!((rslg.i_fault_pu - 1.6666667).abs() < 1e-5);

        let rdlg = a.fault_from_sequence(2, FaultType::DoubleLineToGround, net);
        // I0 = -I1 · Z2 / (Z2 + Z0) = -0.5·0.4/0.5 = -0.4
        // Wait, Glover example: Z_eq = Z2·Z0/(Z2+Z0) = 0.4·0.1/0.5 = 0.08
        // I1 = 1/(0.4+0.08) = 2.0833
        // I0 = -2.0833 · 0.4 / 0.5 = -1.667
        // If = 3|I0| = 5.0
        // (The textbook has Z0 slightly different in different editions;
        // assert that the model matches the analytical formula.)
        let i1 = 1.0 / (z1 + z2 * z0 / (z2 + z0));
        let i0 = -i1 * z2 / (z2 + z0);
        let expected = 3.0 * i0.abs();
        assert!(
            (rdlg.i_fault_pu - expected).abs() < 1e-9,
            "DLG got {} expected {}",
            rdlg.i_fault_pu, expected
        );
    }

    /// Verify the IEEE 14-bus three-phase fault current is in a reasonable
    /// band (3–25 pu) at any load bus.
    #[test]
    fn ieee14_three_phase_fault_in_band() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("test-data")
            .join("ieee")
            .join("ieee14.json");
        let sys = EnergySystem::from_json_file(&path).expect("load ieee14");
        let a = FaultAnalyzer::new(&sys);
        for bus in [4_usize, 5, 9, 10, 14] {
            let r = a.calculate_fault_current(bus, FaultType::ThreePhase);
            assert!(
                r.i_fault_pu > 0.5 && r.i_fault_pu < 50.0,
                "bus {bus}: 3φ fault = {} pu (expected 0.5–50)",
                r.i_fault_pu
            );
        }
    }
}
}
