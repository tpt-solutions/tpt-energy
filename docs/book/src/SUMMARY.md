# Summary

[TPT Energy](introduction.md) is a comprehensive Rust toolkit for power-systems engineering and energy-systems modeling. This book walks you through the architecture and each crate.

# Introduction

- [Introduction](introduction.md)
- [Quick Start](quick-start.md)
- [Architecture](architecture.md)
- [Crate Status](crate-status.md)

# Tutorials

- [Tutorial: 14-bus Power Flow](tutorial-powerflow.md)
- [Tutorial: Solar + Battery Microgrid](tutorial-microgrid.md)
- [Worked Example: Contingency Analysis](worked-example-contingency.md)
- [Worked Example: Battery Sizing](worked-example-battery-sizing.md)

# Core Crates

- [`tpt-nrg-core`](crate-core.md) — `EnergySystem` data model and JSON (de)serialisation.
- [`tpt-nrg-topology`](crate-topology.md) — Graph algorithms and Y-bus builder.
- [`tpt-nrg-timeseries`](crate-timeseries.md) — Time-series containers and resampling.

# Resource Crates

- [`tpt-nrg-solar`](crate-solar.md) — Solar position, clear-sky, POA, PV output.
- [`tpt-nrg-wind`](crate-wind.md) — Vertical profile, Weibull, wake models.
- [`tpt-nrg-hydro` / `tpt-nrg-load` / `tpt-nrg-thermal-storage`](crate-resource-others.md)

# Grid Crates

- [`tpt-nrg-powerflow`](crate-powerflow.md) — AC/DC power flow solvers.
- [`tpt-nrg-fault`](crate-fault.md) — Symmetrical-components short-circuit.

# Storage & Microgrid

- [`tpt-nrg-battery` / `tpt-nrg-hydrogen`](crate-storage.md)
- [`tpt-nrg-der` / `tpt-nrg-islanding` / `tpt-nrg-vpp`](crate-microgrid.md)

# Dispatch & Economics

- [`tpt-nrg-unit-commitment` / `tpt-nrg-economic-dispatch` / `tpt-nrg-reserve`](crate-dispatch.md)
- [`tpt-nrg-lcoe` / `tpt-nrg-market` / `tpt-nrg-carbon`](crate-economics.md)

# Interop & Distribution

- [`tpt-nrg-interop`](crate-interop.md) — MATPOWER, PSS/E, CIM, YAML, and CSV.
- [`tpt-nrg-viz`](crate-viz.md) — SVG single-line diagram and heatmap.
- [`tpt-nrg-cli`](crate-cli.md) — the `tpt-nrg` command-line interface.
- [`tpt-nrg-python`](https://github.com/tpt-solutions/tpt-energy/tree/master/crates/bindings/tpt-nrg-python) — Python bindings.

# Advanced

- [`tpt-nrg-state-estimation` / `tpt-nrg-protection` / `tpt-nrg-wasm`](crate-advanced.md)
- [Building for the browser](wasm-build.md)

# Reference

- [RFCs](https://github.com/tpt-solutions/tpt-energy/tree/master/rfcs) (repository `rfcs/` directory)
- [License audit](reference/license-audit.md)
- [IEC 61850 / 61970 standards review](reference/iec-standards.md)
- [NERC standards review](reference/nerc-standards.md)
