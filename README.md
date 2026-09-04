# tpt-energy

> Power systems, resource modeling, storage, dispatch, and grid economics — pure Rust.

**Org:** TPT Solutions · **License:** MIT OR Apache-2.0 · **Repo:** `tpt-energy`

TPT Energy is a comprehensive, open-source toolkit for power-systems engineering
and energy-systems modeling, written in Rust and built on the TPT substrate
crates (`tpt-math`, `tpt-engineering`, `tpt-science`). It is designed for
research, planning, dispatch, real-time grid control, and in-browser
exploration via WebAssembly.

## Energy Cycle

TPT Energy follows the **Energy Cycle**: resources → grid → storage → microgrid
→ dispatch → economics, all of which integrate back into the TPT substrate
(materials degradation → device physics → grid dispatch).

```mermaid
flowchart LR
    A[Resource<br>solar · wind · hydro · load] --> B[Grid<br>power flow · fault · state est · protection]
    B --> C[Storage<br>battery · hydrogen · thermal]
    C --> D[Microgrid<br>DER · islanding · VPP]
    D --> E[Dispatch<br>unit commitment · economic · reserves]
    E --> F[Economics<br>LCOE · market · carbon]
    F --> A
```

## Crate Status

| Crate                             | Description                                  | Status   |
|-----------------------------------|----------------------------------------------|----------|
| `tpt-nrg-core`                    | Energy system data model                     | Stable   |
| `tpt-nrg-topology`                | Graph & Y-bus construction                   | Stable   |
| `tpt-nrg-timeseries`              | Load/generation/price profiles               | Stable   |
| `tpt-nrg-wasm`                    | Browser/edge WASM bindings                   | Alpha    |
| `tpt-nrg-solar`                   | Solar position & PV output                   | Stable   |
| `tpt-nrg-wind`                    | Wind power & wake losses                     | Stable   |
| `tpt-nrg-hydro`                   | Hydro power calculation                      | Stable   |
| `tpt-nrg-load`                    | Load forecasting & demand response           | Stable   |
| `tpt-nrg-powerflow`               | AC/DC power flow solvers                     | Stable   |
| `tpt-nrg-fault`                   | Short-circuit (fault) analysis               | Stable   |
| `tpt-nrg-state-estimation`        | DC-approx WLS grid state estimator           | Alpha    |
| `tpt-nrg-protection`              | Relay coordination & protection zones        | Stable   |
| `tpt-nrg-battery`                 | Battery storage & degradation                | Stable   |
| `tpt-nrg-hydrogen`                | Electrolyzer / fuel-cell / H2 storage        | Stable   |
| `tpt-nrg-thermal-storage`         | Thermal storage                              | Stable   |
| `tpt-nrg-der`                     | Distributed energy resources                 | Stable   |
| `tpt-nrg-islanding`               | Loss-of-mains detection & resync             | Stable   |
| `tpt-nrg-vpp`                     | Virtual power plant aggregation              | Stable   |
| `tpt-nrg-unit-commitment`         | Priority-list UC (MILP behind `substrate`)   | Alpha    |
| `tpt-nrg-economic-dispatch`       | Economic dispatch & storage arbitrage        | Stable   |
| `tpt-nrg-reserve`                 | Spinning & contingency reserve               | Stable   |
| `tpt-nrg-lcoe`                    | LCOE / NPV / IRR                             | Stable   |
| `tpt-nrg-market`                  | Market signal modeling                       | Stable   |
| `tpt-nrg-carbon`                  | Carbon intensity                             | Stable   |

See [`docs/book/src/crate-status.md`](docs/book/src/crate-status.md) for
detailed maturity notes per crate.

## Quick Start

```toml
# Cargo.toml
[dependencies]
tpt-nrg-core = "0.1"
tpt-nrg-powerflow = "0.1"
```

```rust
use tpt_nrg_core::EnergySystem;
use tpt_nrg_powerflow::{PowerFlowSolver, PowerFlowMethod};

let system = EnergySystem::from_json(include_str!("ieee14.json"))?;
let solver = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson);
let result = solver.solve(&system)?;
println!("{:?}", result);
```

## Examples

- `examples/ieee-14-bus-powerflow` — solve the IEEE 14-bus case
- `examples/solar-farm-layout` — solar position + PV output profile
- `examples/wind-farm-wake` — Jensen wake model
- `examples/microgrid-islanding` — islanding event + battery dispatch
- `examples/battery-arbitrage` — storage arbitrage from price forecast

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at
your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
