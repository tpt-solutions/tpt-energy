# `tpt-nrg-powerflow`

AC and DC power flow solvers. Validated against the IEEE 14-bus and 30-bus MATPOWER cases to within 1% of published results.

## Solvers

| Method | Iterations | Best for |
|--------|-----------|----------|
| `NewtonRaphson` | 3–7 (typical) | Small/medium networks, default choice. |
| `GaussSeidel` | 50+ | Teaching, very small networks. |
| `FastDecoupled` | 5–20 | Large networks where NR Jacobian is expensive. |
| `DcPowerFlow` | 1 (linear) | Bulk dispatch studies, screening. |

## Usage

```rust,no_run
use tpt_nrg_core::EnergySystem;
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};

let system = EnergySystem::from_json(include_str!("../../../test-data/ieee/ieee14.json"))?;
let solver = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson)
    .with_max_iterations(50)
    .with_tolerance_pu(1e-6);
let result = solver.solve(&system)?;
assert!(result.converged);
```

## Result fields

- `converged: bool`
- `iterations: usize`
- `bus_voltage_magnitude_pu: Vec<f64>`
- `bus_voltage_angle_rad: Vec<f64>`
- `branch_flows: Vec<BranchFlow>`
- `total_losses_mw: f64`

## Errors

`PowerFlowError` distinguishes:

- Non-convergence (with iteration count and final mismatch).
- Singular Y-bus (zero-impedance loops).
- Missing slack bus.
- Invalid input (negative impedances, out-of-range setpoints).

## IEEE 57-bus caveat

The NR solver does not converge from a flat start on IEEE 57-bus in this version (high angle spread, off-nominal transformers). Use `DcPowerFlow` for the 57-bus case; full non-linear convergence will return when Q-limit handling is added (RFC 0001).
