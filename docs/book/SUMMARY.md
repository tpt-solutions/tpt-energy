# Summary

[TPT Energy](../README.md) is a comprehensive Rust toolkit for power-systems engineering and energy-systems modeling. This book walks you through the architecture and each crate.

# Architecture

TPT Energy follows the **Energy Cycle**:

```mermaid
flowchart LR
    A[Resource] --> B[Grid]
    B --> C[Storage]
    C --> D[Microgrid]
    D --> E[Dispatch]
    E --> F[Economics]
    F --> A
```

## Crates

| Crate | Description |
|-------|-------------|
| `tpt-nrg-core` | The `EnergySystem` data model. |
| `tpt-nrg-topology` | Graph & Y-bus construction. |
| `tpt-nrg-timeseries` | Time-series containers. |
| `tpt-nrg-powerflow` | AC/DC power flow solvers. |
| `tpt-nrg-fault` | Short-circuit (fault) analysis. |
| `tpt-nrg-solar` | Solar position & PV output. |
| `tpt-nrg-wind` | Wind power & wake models. |
| `tpt-nrg-hydro` | Hydroelectric power. |
| `tpt-nrg-load` | Load forecasting. |
| `tpt-nrg-battery` | Battery storage. |
| `tpt-nrg-hydrogen` | H₂ storage. |
| `tpt-nrg-thermal-storage` | Thermal storage. |
| `tpt-nrg-der` | Distributed energy resources. |
| `tpt-nrg-islanding` | Loss-of-mains detection. |
| `tpt-nrg-vpp` | Virtual power plant. |
| `tpt-nrg-unit-commitment` | Unit commitment. |
| `tpt-nrg-economic-dispatch` | Economic dispatch & arbitrage. |
| `tpt-nrg-reserve` | Operating reserve. |
| `tpt-nrg-lcoe` | LCOE / NPV / IRR. |
| `tpt-nrg-market` | Market signals. |
| `tpt-nrg-carbon` | Carbon intensity. |
| `tpt-nrg-wasm` | WebAssembly bindings. |

# Quick Start

```toml
[dependencies]
tpt-nrg-core = "0.1"
tpt-nrg-powerflow = "0.1"
```

```rust
use tpt_nrg_core::EnergySystem;
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};

let system = EnergySystem::from_json(include_str!("ieee14.json"))?;
let solver = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson);
let result = solver.solve(&system)?;
println!("losses = {} MW", result.total_losses_mw);
```
