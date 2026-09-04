# Crate Status

Per-crate maturity table. Statuses follow the conventions:

- **Stable** — public API is stable, semantics are tested against golden data, suitable for production use.
- **Alpha** — API may change in a minor release, but the crate compiles, tests pass, and the documented usage patterns are correct.
- **Planned** — directory exists and may contain stubs, but no real implementation yet.

| Crate | Status | Notes |
|-------|--------|-------|
| `tpt-nrg-core` | Stable | `EnergySystem` data model, JSON (de)serialisation. |
| `tpt-nrg-topology` | Stable | Island detection, shortest path, Y-bus builder. |
| `tpt-nrg-timeseries` | Stable | Uniform and irregular time-series, resampling, interpolation. |
| `tpt-nrg-powerflow` | Stable | NR / Gauss-Seidel / Fast-Decoupled / DC. Validated against MATPOWER IEEE 14-bus and 30-bus. |
| `tpt-nrg-fault` | Stable | Symmetrical / line-to-line / line-to-ground / double-line-to-ground. |
| `tpt-nrg-state-estimation` | Alpha | DC-approximation WLS only; full non-linear WLS planned. |
| `tpt-nrg-protection` | Stable | IEC 60255-151 inverse-time curves and coordination checks. |
| `tpt-nrg-solar` | Stable | SPA-equivalent position, Ineichen clear-sky, POA transposition, NOCT PV output. |
| `tpt-nrg-wind` | Stable | Log/power-law vertical extrapolation, Weibull PDF, Jensen / Frandsen / eddy-viscosity wake models. |
| `tpt-nrg-hydro` | Stable | Head × flow × efficiency model. |
| `tpt-nrg-load` | Stable | Diurnal/weekly shape, temperature sensitivity, price elasticity. |
| `tpt-nrg-battery` | Stable | SoC tracking, round-trip efficiency, cycle/calendar degradation. |
| `tpt-nrg-hydrogen` | Stable | Electrolyzer + fuel cell + tank model. |
| `tpt-nrg-thermal-storage` | Stable | Thermal SoC tracking. |
| `tpt-nrg-der` | Stable | DER asset aggregation, microgrid controller state. |
| `tpt-nrg-islanding` | Stable | IEEE 1547 islanding detection, controlled transition, resync. |
| `tpt-nrg-vpp` | Stable | VPP aggregation and pro-rata dispatch. |
| `tpt-nrg-unit-commitment` | Alpha | Priority-list heuristic. Full MILP behind `substrate` feature. |
| `tpt-nrg-economic-dispatch` | Stable | Merit-order dispatch, λ iteration, storage arbitrage. |
| `tpt-nrg-reserve` | Stable | Spinning and contingency reserve. |
| `tpt-nrg-lcoe` | Stable | LCOE, NPV, IRR. |
| `tpt-nrg-market` | Stable | Market-signal time-series helpers. |
| `tpt-nrg-carbon` | Stable | Emission-factor lookup, dispatch-weighted carbon intensity. |
| `tpt-nrg-wasm` | Alpha | Native shim compiles; `wasm-bindgen` bindings require the `wasm32-unknown-unknown` toolchain. |

## Cross-cutting

| Concern | Status |
|---------|--------|
| CI (fmt + clippy + test on Linux/Windows/macOS) | Stable |
| License audit (`cargo deny`) | Stable |
| Benchmarks | Planned (`benches/` stubs exist) |
| Cross-repo integration tests | Planned (substrate crates live in separate repos) |
| Standards compliance (IEC 61850 / 61970, NERC) | Planned |
| crates.io publish | Planned |
