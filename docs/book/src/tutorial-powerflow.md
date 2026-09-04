# Tutorial: IEEE 14-bus Power Flow

The IEEE 14-bus test case is the canonical "hello world" of power-flow analysis. TPT Energy ships the case as `test-data/ieee/ieee14.json` and the solver converges in 3–5 Newton–Raphson iterations from a flat start.

## Setup

```toml
[dependencies]
tpt-nrg-core = "0.1"
tpt-nrg-powerflow = "0.1"
anyhow = "1"
```

## Code

```rust,no_run
use tpt_nrg_core::EnergySystem;
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};

fn main() -> anyhow::Result<()> {
    let json = include_str!("../../../test-data/ieee/ieee14.json");
    let system = EnergySystem::from_json(json)?;

    let solver = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson)
        .with_max_iterations(50)
        .with_tolerance_pu(1e-6);
    let result = solver.solve(&system)?;

    assert!(result.converged);
    println!("converged in {} iters, losses = {:.3} MW",
        result.iterations, result.total_losses_mw);

    // Print voltages
    for (i, (vm, va)) in result.bus_voltage_magnitude_pu.iter()
        .zip(result.bus_voltage_angle_rad.iter())
        .enumerate()
    {
        println!("bus {:2}: |V|={:.4} pu, θ={:+.4} rad",
            system.buses[i].id, vm, va);
    }
    Ok(())
}
```

## Expected output

```
converged in 4 iters, losses = 13.59 MW
bus  1: |V|=1.0600 pu, θ=+0.0000 rad
bus  2: |V|=1.0450 pu, θ=-0.0867 rad
bus  3: |V|=1.0100 pu, θ=-0.2221 rad
bus  4: |V|=1.0179 pu, θ=-0.1796 rad
...
```

## What is happening under the hood

1. `EnergySystem::from_json` deserialises the bus, branch, and generator data.
2. The solver builds the per-unit Y-bus: `Y[i,j] = -1/(r+jx)` for each line, `Y[i,i] = -Σ_j Y[i,j]` plus any shunt.
3. Newton–Raphson iteration:
   - Compute the power mismatch `ΔP, ΔQ` at every PV and PQ bus.
   - Build the Jacobian `[∂P/∂θ, ∂P/∂|V|; ∂Q/∂θ, ∂Q/∂|V|]`.
   - Solve `J · [Δθ; Δ|V|] = [ΔP; ΔQ]`.
   - Update the state.
4. Stop when `max(|ΔP|, |ΔQ|) < tolerance`.
5. Compute branch flows and total system losses.

## Trying other methods

```rust,no_run
use tpt_nrg_powerflow::PowerFlowMethod;

// Fast-Decoupled: same Newton–Raphson Jacobian, but with constant
// B' and B" submatrices. Faster per iteration, more iterations.
let solver = PowerFlowSolver::new(PowerFlowMethod::FastDecoupled);

// DC power flow: ignore reactive power entirely, linear solve.
let solver = PowerFlowSolver::new(PowerFlowMethod::DcPowerFlow);
```

For systems of more than ~1000 buses, `PowerFlowMethod::FastDecoupled` is usually the right trade-off between accuracy and speed.

## See also

- The `examples/ieee-14-bus-powerflow.rs` example in the workspace.
- The `test-data/golden/powerflow/ieee-14-bus.json` reference solution.
