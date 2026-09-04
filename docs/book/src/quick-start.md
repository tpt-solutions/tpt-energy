# Quick Start

Add TPT Energy to your `Cargo.toml`:

```toml
[dependencies]
tpt-nrg-core = "0.1"
tpt-nrg-powerflow = "0.1"
```

The smallest useful program: parse the bundled IEEE 14-bus system and run a Newton–Raphson power flow.

```rust,no_run
use tpt_nrg_core::EnergySystem;
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};

fn main() -> anyhow::Result<()> {
    // The IEEE 14-bus test case ships as a fixture in the workspace.
    let json = include_str!("../../../test-data/ieee/ieee14.json");
    let system = EnergySystem::from_json(json)?;

    let solver = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson);
    let result = solver.solve(&system)?;

    println!("converged in {} iterations", result.iterations);
    println!("total losses: {:.3} MW", result.total_losses_mw);
    for (i, v) in result.bus_voltage_magnitude_pu.iter().enumerate() {
        println!("  bus {:2}: |V| = {:.4} pu,  θ = {:+.4} rad",
            i + 1, v, result.bus_voltage_angle_rad[i]);
    }
    Ok(())
}
```

## What this does

1. Parses the bus/branch/generator data from JSON into an `EnergySystem`.
2. Builds the Y-bus admittance matrix internally.
3. Iterates Newton–Raphson from a flat start (1.0 pu, 0 rad on every bus).
4. Returns the per-bus voltage magnitude and angle, branch flows, and system losses.

## Next steps

- [Tutorial: 14-bus Power Flow](tutorial-powerflow.md) — full walk-through with expected results.
- [Tutorial: Solar + Battery Microgrid](tutorial-microgrid.md) — end-to-end dispatch.
