# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `run_powerflow_json` — Newton–Raphson power flow from a JSON `EnergySystem` string.
- `validate_system_json` structural validation.
- `economic_dispatch_json`, `unit_commitment_json`, `lcoe_json`, and
  `carbon_intensity_json`, extending the JSON surface past power flow.
- `visualize_json` — an SVG single-line diagram and heatmap via `tpt-nrg-viz`.
- `WasmErrorKind` — the stable slug carried by every `WasmError`.
- `control` module: `ControlConfig`, `Measurements`, and `ControlDecision`,
  with a real droop-control law (P/f and Q/V droops, battery headroom
  limits, dispatchable setpoints, and load shedding).
- `WasmPowerFlowResult` gained `branch_flow_mva` and `branch_loading`.
- `MicrogridAsset` gained `energy_capacity_mwh` (defaults to one hour of
  rated power) and the `is_renewable` / `is_battery` / `is_dispatchable` /
  `is_load` classifiers.
- `#[wasm_bindgen]` exports for dispatch, commitment, LCOE, carbon, and
  visualization behind the `wasm` feature.
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.
- A publishable npm package: `npm/package.json` with the hand-written
  metadata `wasm-pack` does not generate, `npm/tpt_nrg_wasm.d.ts` checked in
  and verified by `tools/build-npm-package.sh --check`, and `npm/README.md`
  for the registry page.

### Changed
- The crate now declares `crate-type = ["cdylib", "rlib"]`. `wasm-pack`
  requires the `cdylib`, so the package could not be built before; the `rlib`
  half keeps the crate usable and testable from the workspace in native
  builds.
- `# Errors` documentation sections on the JSON-entry-point functions.
- **`WasmError` is a breaking change:** it is now a struct with
  `{ kind: WasmErrorKind, message: String }` instead of a `#[serde(tag)]`
  enum, so the JSON shape is stable and easy to mirror in TypeScript.
- Every `#[wasm_bindgen]` export now rejects with a `WasmError` *object*
  rather than a stringified `JsValue`, so a JavaScript caller can
  `switch (err.kind)`.
- `ControlAction` gained `kind`, `new_state_of_charge`, and `action`
  (one of `hold`, `charge`, `discharge`, `setpoint`, `shed`), so a caller
  can tell a hold from a charge and apply the new state of charge.
- `microgrid_step_json` takes an optional third `config_json` argument
  (ignored when absent, so existing callers keep working). It also now
  returns a `ControlDecision` with the utility interchange, the residual
  deficit, the load-shed fraction, and the grid-forming flag, instead of a
  bare array of actions.
- `Measurements` has a hand-written `Default` so `grid_connected` defaults
  to `true` in Rust exactly as it does through `serde`.

### Fixed
- `microgrid_step_json` no longer returns an empty action list. It now
  implements the documented control law and reports a setpoint for every
  asset.

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
