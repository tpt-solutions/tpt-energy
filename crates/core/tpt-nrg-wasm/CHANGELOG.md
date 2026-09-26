# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `run_powerflow_json` — Newton–Raphson power flow from a JSON `EnergySystem` string.
- `validate_system_json` structural validation.
- `microgrid_step_json` control-loop iteration shim.
- `WasmPowerFlowResult`, `WasmError`, `ControlAction`, `MicrogridState` binding types.
- `WasmMicrogridController` bindings behind the `wasm` feature (compiles on `wasm32-unknown-unknown`).
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

### Changed
- `# Errors` documentation sections on the JSON-entry-point functions.

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
