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
        let bus_index: Vec<Option<usize>> = {
            let mut idx = vec![None; n + 1];
            for (i, b) in self.system.buses.iter().enumerate() {
                if b.id <= n {
                    idx[b.id] = Some(i);
                }
            }
            idx
        };
        let mut y = AdmittanceMatrix::zeros(n);

        for b in &self.system.buses {
            if let Some(i) = bus_index[b.id] {
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
                let tap_sq = tap * tap;
                // y / |t|² and y / t (complex division)
                y.g[i * n + i] += g_series / tap_sq;
                y.b[i * n + i] += b_series / tap_sq + bc_half;
                y.g[j * n + j] += g_series;
                y.b[j * n + j] += b_series + bc_half;
                // -y / t* : multiply -y by t
                let (gt, bt) = mul_complex(-g_series, -b_series, tap, shift);
                y.g[i * n + j] += gt;
                y.b[i * n + j] += bt;
                // -y / t : multiply -y by t*
                let (gt2, bt2) = mul_complex(-g_series, -b_series, tap, -shift);
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
}
