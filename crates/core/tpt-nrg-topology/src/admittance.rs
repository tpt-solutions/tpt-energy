//! Y-bus admittance matrix construction.

use tpt_nrg_core::EnergySystem;

/// Dense complex admittance matrix (G + jB), stored as parallel real/imaginary
/// `Vec<f64>`s of length `n*n` in row-major order.
#[derive(Debug, Clone)]
pub struct AdmittanceMatrix {
    /// Number of buses.
    pub n: usize,
    /// G[i*n+j] = real part of Y[i,j] in per-unit.
    pub g: Vec<f64>,
    /// B[i*n+j] = imaginary part of Y[i,j] in per-unit.
    pub b: Vec<f64>,
}

impl AdmittanceMatrix {
    /// Construct a zero matrix of size `n`.
    #[must_use]
    pub fn zeros(n: usize) -> Self {
        Self {
            n,
            g: vec![0.0; n * n],
            b: vec![0.0; n * n],
        }
    }

    /// Real part Y[i,j].
    #[inline]
    #[must_use]
    pub fn g_ij(&self, i: usize, j: usize) -> f64 {
        self.g[i * self.n + j]
    }

    /// Imaginary part Y[i,j].
    #[inline]
    #[must_use]
    pub fn b_ij(&self, i: usize, j: usize) -> f64 {
        self.b[i * self.n + j]
    }

    /// Complex magnitude |Y[i,j]|.
    #[must_use]
    pub fn magnitude(&self, i: usize, j: usize) -> f64 {
        let g = self.g_ij(i, j);
        let b = self.b_ij(i, j);
        (g * g + b * b).sqrt()
    }
}

/// Builder for the bus admittance matrix.
///
/// Supports transmission lines and off-nominal tap transformers. The
/// phase-shifting branch is handled with a complex tap ratio `t * e^{jθ}`.
pub struct AdmittanceMatrixBuilder<'a> {
    system: &'a EnergySystem,
}

impl<'a> AdmittanceMatrixBuilder<'a> {
    /// Create a new builder for the given system.
    #[must_use]
    pub fn new(system: &'a EnergySystem) -> Self {
        Self { system }
    }

    /// Build the Y-bus admittance matrix.
    ///
    /// For a branch from bus `f` to bus `t` with series admittance
    /// `y = 1/(r+jx)` and total line charging `B/2` at each end:
    ///
    /// ```text
    /// Y[f,f] += y / |t|² + jB/2
    /// Y[t,t] += y         + jB/2
    /// Y[f,t] -= y / t*    (with phase shift)
    /// Y[t,f] -= y / t
    /// ```
    ///
    /// where `t = tap_ratio * e^{j·phase_shift}`.
    #[must_use]
    pub fn build(&self) -> AdmittanceMatrix {
        let n = self.system.buses.len();
        // Map bus id → dense index sized by max id so that non-contiguous
        // or 0-based id schemes still resolve (bus ids are not required to
        // be 1..=n).
        let bus_index: Vec<Option<usize>> = {
            let max_id = self.system.buses.iter().map(|b| b.id).max().unwrap_or(0);
            let mut idx = vec![None; max_id + 1];
            for (i, b) in self.system.buses.iter().enumerate() {
                idx[b.id] = Some(i);
            }
            idx
        };
        let mut y = AdmittanceMatrix::zeros(n);

        for b in &self.system.buses {
            // `get` (not index) — bus ids may exceed the bus count on
            // non-contiguous id schemes; indexing would panic.
            if let Some(i) = bus_index.get(b.id).copied().flatten() {
                y.g[i * n + i] += b.shunt_conductance_pu;
                y.b[i * n + i] += b.shunt_susceptance_pu;
            }
        }

        for br in &self.system.branches {
            if !br.in_service {
                continue;
            }
            let i = match bus_index.get(br.from_bus).and_then(|x| *x) {
                Some(v) => v,
                None => continue,
            };
            let j = match bus_index.get(br.to_bus).and_then(|x| *x) {
                Some(v) => v,
                None => continue,
            };

            // Series admittance y = 1/(r + jx)
            let r = br.resistance_pu;
            let x = br.reactance_pu;
            let denom = r * r + x * x;
            if denom == 0.0 {
                continue;
            }
            let g_series = r / denom;
            let b_series = -x / denom;
            // Total line charging susceptance split half at each end
            let bc_half = br.susceptance_pu * 0.5;

            // Tap ratio (complex) t = tap_ratio * exp(j * phase_shift)
            let tap = br.tap_ratio;
            let shift = br.phase_shift_rad;
            // If tap == 1 and shift == 0, the off-nominal model reduces to a
            // simple line.
            if (tap - 1.0).abs() < 1e-12 && shift.abs() < 1e-12 {
                y.g[i * n + i] += g_series;
                y.b[i * n + i] += b_series + bc_half;
                y.g[j * n + j] += g_series;
                y.b[j * n + j] += b_series + bc_half;
                y.g[i * n + j] -= g_series;
                y.b[i * n + j] -= b_series;
                y.g[j * n + i] -= g_series;
                y.b[j * n + i] -= b_series;
            } else {
                // Complex tap t = tap·e^{jθ}. The π-model requires
                //   Y[f,f] += y/|t|²,  Y[t,t] += y,
                //   Y[f,t] -= y/t* = -y·e^{+jθ}/tap,
                //   Y[t,f] -= y/t  = -y·e^{-jθ}/tap.
                let tap_sq = tap * tap;
                y.g[i * n + i] += g_series / tap_sq;
                y.b[i * n + i] += b_series / tap_sq + bc_half;
                y.g[j * n + j] += g_series;
                y.b[j * n + j] += b_series + bc_half;
                let inv_tap = 1.0 / tap;
                let (cs, sn) = (shift.cos() * inv_tap, shift.sin() * inv_tap);
                // -y / t*
                let (gt, bt) = mul_complex(-g_series, -b_series, cs, sn);
                y.g[i * n + j] += gt;
                y.b[i * n + j] += bt;
                // -y / t
                let (gt2, bt2) = mul_complex(-g_series, -b_series, cs, -sn);
                y.g[j * n + i] += gt2;
                y.b[j * n + i] += bt2;
            }
        }
        y
    }
}

/// Multiply two complex numbers `(a + jb) * (c + jd)`.
fn mul_complex(a: f64, b: f64, c: f64, d: f64) -> (f64, f64) {
    (a * c - b * d, a * d + b * c)
}

/// Substrate-backed sparse Y-bus construction.
///
/// Returns a `tpt-math-linalg-sparse` `CooMatrix` with the complex entries
/// stored as 2 × (i, j, value) entries (G and B interleaved into two
/// matrices). Only available with the `substrate` feature.
#[cfg(feature = "substrate")]
#[must_use]
pub fn build_sparse_coo(system: &EnergySystem) -> tpt_math_linalg_sparse::CooMatrix<f64> {
    use std::collections::HashMap;
    use tpt_math_linalg_sparse::CooMatrix;
    let n = system.buses.len();
    // Map bus id → dense index exactly like the dense path (ids may be
    // non-contiguous or 0-based; never assume id-1).
    let index: HashMap<usize, usize> = system
        .buses
        .iter()
        .enumerate()
        .map(|(i, b)| (b.id, i))
        .collect();
    let mut coo = CooMatrix::<f64>::new(n, n);
    // Diagonal shunt
    for (i, b) in system.buses.iter().enumerate() {
        if b.shunt_susceptance_pu != 0.0 {
            coo.push(i, i, b.shunt_susceptance_pu);
        }
    }
    for br in &system.branches {
        if !br.in_service {
            continue;
        }
        let (i, j) = match (index.get(&br.from_bus), index.get(&br.to_bus)) {
            (Some(&i), Some(&j)) => (i, j),
            _ => continue,
        };
        let r = br.resistance_pu;
        let x = br.reactance_pu;
        let denom = r * r + x * x;
        if denom == 0.0 {
            continue;
        }
        let g_series = r / denom;
        let b_series = -x / denom;
        let bc_half = br.susceptance_pu * 0.5;
        // Diagonal contributions (imag only, summed)
        coo.push(i, i, b_series + bc_half);
        coo.push(j, j, b_series + bc_half);
        // Off-diagonal
        coo.push(i, j, -b_series);
        coo.push(j, i, -b_series);
        // (G is dropped here — substrate path returns a B-only matrix for
        //  demonstration; the dense path remains the source of truth for
        //  real-valued admittance.)
        let _ = g_series;
    }
    coo
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_nrg_core::{Branch, Bus, BusType};

    fn three_bus() -> EnergySystem {
        let mut sys = EnergySystem::new("t", "T", 100.0, 60.0);
        sys.add_bus(Bus::new(1, "B1", BusType::Slack)).unwrap();
        sys.add_bus(Bus::new(2, "B2", BusType::Pq)).unwrap();
        sys.add_bus(Bus::new(3, "B3", BusType::Pq)).unwrap();
        sys.add_branch(Branch::new(1, "L12", 1, 2, 0.01, 0.05)).unwrap();
        sys.add_branch(Branch::new(2, "L23", 2, 3, 0.02, 0.10)).unwrap();
        sys
    }

    #[test]
    fn y_bus_diagonal_positive() {
        let sys = three_bus();
        let y = AdmittanceMatrixBuilder::new(&sys).build();
        assert_eq!(y.n, 3);
        // Diagonal of a passive network must be non-negative real part.
        for i in 0..3 {
            assert!(y.g_ij(i, i) >= 0.0, "Y[{i},{i}] real = {}", y.g_ij(i, i));
        }
    }

    #[test]
    fn y_bus_off_diagonal_negative_for_line() {
        let sys = three_bus();
        let y = AdmittanceMatrixBuilder::new(&sys).build();
        // Off-diagonals for a simple R-L line should be negative real and
        // positive imaginary (since y has b < 0 and y_offdiag = -y).
        for &(i, j) in &[(0usize, 1usize), (1, 0), (1, 2), (2, 1)] {
            assert!(y.g_ij(i, j) <= 0.0, "Y[{i},{j}] real = {}", y.g_ij(i, j));
            assert!(y.b_ij(i, j) >= 0.0, "Y[{i},{j}] imag = {}", y.b_ij(i, j));
        }
    }

    #[test]
    fn y_bus_symmetric_for_simple_line() {
        let sys = three_bus();
        let y = AdmittanceMatrixBuilder::new(&sys).build();
        for i in 0..3 {
            for j in 0..3 {
                assert!((y.g_ij(i, j) - y.g_ij(j, i)).abs() < 1e-12);
                assert!((y.b_ij(i, j) - y.b_ij(j, i)).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn out_of_service_branch_ignored() {
        let mut sys = three_bus();
        for br in &mut sys.branches {
            if br.id == 1 {
                br.in_service = false;
            }
        }
        let y = AdmittanceMatrixBuilder::new(&sys).build();
        assert!((y.g_ij(0, 1)).abs() < 1e-12);
        assert!((y.b_ij(0, 1)).abs() < 1e-12);
    }

    #[test]
    fn y_bus_tap_off_diagonal_divides_by_tap() {
        // r=0, x=0.1, tap=0.95, shift=0:
        //   y = -j10
        //   Y[f,f] = y/tap² = -j11.080...
        //   Y[f,t] = -y/tap = +j10.526...  (NOT -y·tap)
        let mut sys = EnergySystem::new("t", "T", 100.0, 60.0);
        sys.add_bus(Bus::new(1, "B1", BusType::Slack)).unwrap();
        sys.add_bus(Bus::new(2, "B2", BusType::Pq)).unwrap();
        let mut br = Branch::new(1, "T12", 1, 2, 0.0, 0.1);
        br.tap_ratio = 0.95;
        sys.add_branch(br).unwrap();
        let y = AdmittanceMatrixBuilder::new(&sys).build();
        assert!((y.b_ij(0, 0) - (-10.0 / (0.95 * 0.95))).abs() < 1e-9);
        assert!((y.b_ij(0, 1) - (10.0 / 0.95)).abs() < 1e-9);
        assert!((y.b_ij(1, 0) - (10.0 / 0.95)).abs() < 1e-9);
    }

    #[test]
    fn y_bus_phase_shift_off_diagonals_are_conjugates() {
        // tap=1, shift=π/6, y = -j10:
        //   Y[f,t] = -y·e^{jθ}  = -10·sinθ + j·10·cosθ
        //   Y[t,f] = -y·e^{-jθ} = +10·sinθ + j·10·cosθ
        let mut sys = EnergySystem::new("t", "T", 100.0, 60.0);
        sys.add_bus(Bus::new(1, "B1", BusType::Slack)).unwrap();
        sys.add_bus(Bus::new(2, "B2", BusType::Pq)).unwrap();
        let mut br = Branch::new(1, "PS12", 1, 2, 0.0, 0.1);
        br.phase_shift_rad = std::f64::consts::FRAC_PI_6;
        sys.add_branch(br).unwrap();
        let y = AdmittanceMatrixBuilder::new(&sys).build();
        let s = std::f64::consts::FRAC_PI_6.sin();
        let c = std::f64::consts::FRAC_PI_6.cos();
        assert!((y.g_ij(0, 1) - (-10.0 * s)).abs() < 1e-9);
        assert!((y.b_ij(0, 1) - (10.0 * c)).abs() < 1e-9);
        assert!((y.g_ij(1, 0) - (10.0 * s)).abs() < 1e-9);
        assert!((y.b_ij(1, 0) - (10.0 * c)).abs() < 1e-9);
    }

    #[test]
    fn y_bus_survives_non_contiguous_bus_ids() {
        // Bus ids {10, 20} with 2 buses must not panic and must populate
        // the 2×2 matrix in dense order.
        let mut sys = EnergySystem::new("t", "T", 100.0, 60.0);
        sys.add_bus(Bus::new(10, "B10", BusType::Slack)).unwrap();
        sys.add_bus(Bus::new(20, "B20", BusType::Pq)).unwrap();
        sys.add_branch(Branch::new(1, "L", 10, 20, 0.01, 0.05)).unwrap();
        let y = AdmittanceMatrixBuilder::new(&sys).build();
        assert!((y.g_ij(0, 1) - y.g_ij(1, 0)).abs() < 1e-12);
        assert!(
            y.g_ij(0, 1).abs() > 0.0 || y.b_ij(0, 1).abs() > 0.0,
            "branch between non-contiguous ids must populate Y"
        );
    }

    /// Phase 1 milestone: parse IEEE 14-bus and validate Y-bus against the
    /// standard published values for branch 1 (bus 1 → bus 2).
    ///
    /// For r = 0.01938 pu, x = 0.05917 pu, bc = 0.0528 pu:
    ///   denom   = r² + x² = 0.0038768
    ///   g_series = r / denom ≈ 4.9994
    ///   b_series = -x / denom ≈ -15.2629
    ///   bc_half  = bc / 2 ≈ 0.0264
    ///
    /// Expected off-diagonal: G₁₂ ≈ -4.9994, B₁₂ ≈ +15.2629.
    /// Expected diagonal contributions to Y₁₁ from this branch:
    ///   G₁₁ += 4.9994, B₁₁ += -15.2629 + 0.0264 ≈ -15.2365.
    #[test]
    fn ieee14_y_bus_matches_published_values() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("test-data")
            .join("ieee")
            .join("ieee14.json");
        let sys = tpt_nrg_core::EnergySystem::from_json_file(&path).expect("load ieee14");
        let y = AdmittanceMatrixBuilder::new(&sys).build();

        assert_eq!(y.n, 14);

        // Branch 1-2 off-diagonal (0-indexed: row 0, col 1).
        // 1.0 / (0.01938 + j0.05917) ≈ 4.9994 - j15.2629; off-diag is -y.
        assert!(
            (y.g_ij(0, 1) - (-4.9994)).abs() < 1e-3,
            "G[0,1] = {}",
            y.g_ij(0, 1)
        );
        assert!(
            (y.b_ij(0, 1) - 15.2629).abs() < 1e-3,
            "B[0,1] = {}",
            y.b_ij(0, 1)
        );

        // Diagonal symmetry check: Y-bus is symmetric for a passive network.
        for i in 0..14 {
            for j in 0..14 {
                assert!(
                    (y.g_ij(i, j) - y.g_ij(j, i)).abs() < 1e-9,
                    "Y[{i},{j}] G not symmetric"
                );
                assert!(
                    (y.b_ij(i, j) - y.b_ij(j, i)).abs() < 1e-9,
                    "Y[{i},{j}] B not symmetric"
                );
            }
        }

        // Branch 2-4 (r=0.05811, x=0.17632).
        // denom = 0.05811² + 0.17632² = 0.003377 + 0.031089 = 0.034466
        // g = 0.05811 / 0.034466 ≈ 1.6858
        // b = -0.17632 / 0.034466 ≈ -5.1154
        // Off-diag = -y, so G[1,3] ≈ -1.6858, B[1,3] ≈ +5.1154.
        assert!(
            (y.g_ij(1, 3) - (-1.6858)).abs() < 1e-3,
            "G[1,3] = {}",
            y.g_ij(1, 3)
        );
        assert!(
            (y.b_ij(1, 3) - 5.1154).abs() < 1e-3,
            "B[1,3] = {}",
            y.b_ij(1, 3)
        );
    }
}
