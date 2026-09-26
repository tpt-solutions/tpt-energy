# tpt-nrg-wasm

WebAssembly bindings for TPT Energy: a JSON-in / JSON-out surface over the
power-flow solver and the microgrid controller, usable from JavaScript or
TypeScript via `wasm-bindgen`.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `run_powerflow_json` — Newton–Raphson power flow from a JSON
  `EnergySystem` string, returning converged voltages, angles, and losses.
- `validate_system_json` — fast structural validation of a system
  document.
- `microgrid_step_json` — one control-loop iteration over a serialized
  microgrid state (placeholder control logic; see status).
- `WasmMicrogridController` and `ControlAction` `#[wasm_bindgen]`
  bindings behind the `wasm` feature (compile on `wasm32-unknown-unknown`).
- Native shim: the same functions work in plain Rust tests so the rest of
  the workspace does not require a WASM toolchain.

## Installation

```toml
[dependencies]
tpt-nrg-wasm = "0.1"
```

## Usage (native / test shim)

```rust
let system_json = std::fs::read_to_string("ieee14.json")?;
tpt_nrg_wasm::validate_system_json(&system_json)?;

let result = tpt_nrg_wasm::run_powerflow_json(&system_json)?;
println!("converged = {} in {} iterations", result.converged, result.iterations);
```

Building for the browser requires the `wasm32-unknown-unknown` target:

```sh
rustup target add wasm32-unknown-unknown
wasm-pack build crates/core/tpt-nrg-wasm --target web --features wasm
```

See [the WASM build guide](https://github.com/tpt-solutions/tpt-energy/blob/master/docs/book/src/wasm-build.md)
for the full browser pipeline.

## Crates.io metadata

- **Categories**: `science`, `simulation`, `wasm`
- **Keywords**: `power-systems`, `wasm`, `webassembly`, `energy`, `edge`

## Status

**Alpha.** The JSON surface and bindings are tested on the native shim;
browser demos (interactive power flow, edge control) still require a
browser harness repository.

## Testing

```sh
cargo test -p tpt-nrg-wasm --all-features
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
