# Introduction

TPT Energy is a comprehensive Rust toolkit for power-systems engineering and energy-systems modeling.

The toolkit covers the full **Energy Cycle**:

```mermaid
flowchart LR
    A[Resource] --> B[Grid]
    B --> C[Storage]
    C --> D[Microgrid]
    D --> E[Dispatch]
    E --> F[Economics]
    F --> A
```

## Why TPT Energy?

- **One toolkit, full Energy Cycle.** Resource assessment → grid analysis → storage → microgrid → dispatch → economics, all in a single type-safe Rust workspace.
- **Self-contained algorithms with optional substrate.** Newton–Raphson power flow, Weibull PDF, clear-sky irradiance, and MILP solvers ship as zero-dependency native implementations. When the upstream TPT substrate crates are enabled (`substrate` feature), they swap in upstream implementations with the same public API.
- **Standards-aware data model.** The `EnergySystem` mirrors the structure used by MATPOWER and PSS®E (buses, branches, generators, loads, storage in per-unit).
- **WebAssembly bindings.** `tpt-nrg-wasm` exposes a JSON-driven power-flow endpoint for in-browser use.
- **MIT OR Apache-2.0 licensed.** Permissive, no copyleft, no patent clause.

## Who is this for?

- Power-systems engineers prototyping dispatch or storage optimizations.
- Renewable-energy analysts needing transparent, auditable resource models.
- Microgrid and VPP developers running what-if studies.
- Researchers who want a reproducible baseline (no black-box solvers).
- Embedded / browser / edge deployments via WASM or `no_std` paths.

## Crate status

See the [Crate Status](../crate-status.md) chapter for the per-crate maturity table (Stable / Alpha / Planned).

## Where to go next

- [Quick Start](quick-start.md) — a 30-line power flow.
- [Architecture](architecture.md) — the Energy Cycle and crate layout.
- [Tutorial: 14-bus Power Flow](tutorial-powerflow.md) — load IEEE 14, solve, inspect results.
- [Tutorial: Solar + Battery Microgrid](tutorial-microgrid.md) — end-to-end dispatch simulation.
