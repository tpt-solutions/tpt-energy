# Changelog

All notable changes to tpt-energy will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial workspace structure with 24 crates across 6 domains (core, resource,
  grid, storage, microgrid, dispatch, economics).
- `tpt-nrg-core` data model: `EnergySystem`, `Bus`, `Branch`, `Generator`,
  `CostCurve`, `HeatRateCurve`, `PowerCurve` with JSON (de)serialization.
- `tpt-nrg-topology`: graph construction, connected components, shortest
  path, Y-bus admittance matrix builder.
- `tpt-nrg-timeseries`: time-series container, resampling, interpolation.
- `tpt-nrg-powerflow`: Newton–Raphson, Gauss–Seidel, Fast Decoupled, and
  DC power flow solvers.
- `tpt-nrg-fault`: symmetrical-component short-circuit analysis.
- `tpt-nrg-solar`: NREL SPA solar position, clear-sky irradiance, plane-of-
  array, PV output with temperature derating.
- `tpt-nrg-wind`: log/power-law wind profile, Weibull PDF, Jensen/Frandsen/
  Eddy-Viscosity wake models, farm-level power output.
- `tpt-nrg-hydro`: head/flow power calculation.
- `tpt-nrg-load`: load forecasting and price-elastic demand response.
- `tpt-nrg-battery`: SoC-aware charge/discharge, degradation model.
- `tpt-nrg-hydrogen`: electrolyzer + fuel-cell efficiency model.
- `tpt-nrg-der`, `tpt-nrg-islanding`, `tpt-nrg-vpp`: microgrid control.
- `tpt-nrg-unit-commitment`, `tpt-nrg-economic-dispatch`, `tpt-nrg-reserve`:
  dispatch and reserve.
- `tpt-nrg-lcoe`, `tpt-nrg-market`, `tpt-nrg-carbon`: economics.
- IEEE 14/30/57-bus golden test cases.
- CI, license-audit, benchmark, and docs workflows.
