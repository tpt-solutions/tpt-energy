# tpt-nrg-wasm

WebAssembly bindings for TPT Energy: a JSON-in / JSON-out surface over power
flow, economic dispatch, unit commitment, LCOE, carbon intensity, SVG
rendering, and the microgrid controller, usable from JavaScript or TypeScript
via `wasm-bindgen`.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

Every export returns a `WasmError` of the shape `{ kind, message }`, where
`kind` is a stable slug, so a JavaScript caller switches on `err.kind` rather
than parsing text.

- `run_powerflow_json` — Newton–Raphson power flow from a JSON
  `EnergySystem` string, returning converged voltages, angles, losses, and
  per-branch flows and loadings.
- `validate_system_json` — fast structural validation of a system document.
- `economic_dispatch_json`, `unit_commitment_json`, `lcoe_json`,
  `carbon_intensity_json` — the dispatch and economics surface.
- `visualize_json` — an SVG single-line diagram and heatmap.
- `microgrid_step_json` — one control-loop iteration, with real droop
  control (P/f and Q/V droops, battery headroom limits, dispatchable
  setpoints, and load shedding).
- `WasmMicrogridController` and `ControlAction` `#[wasm_bindgen]` bindings
  behind the `wasm` feature (compile on `wasm32-unknown-unknown`).
- Native shim: the same functions work in plain Rust tests so the rest of
  the workspace does not require a WASM toolchain.

## Installation

```toml
[dependencies]
tpt-nrg-wasm = "0.1"
```

## Error kinds

| `kind`            | Meaning                                       |
|-------------------|-----------------------------------------------|
| `json`            | the input was not valid JSON                  |
| `validation`      | it parsed, but is not a well-formed system    |
| `non_convergence` | the solver did not converge                   |
| `infeasible`      | the dispatch problem has no solution          |
| `control`         | the control loop got inconsistent measurements |
| `serialization`   | the result could not be encoded               |

## Usage (native / test shim)

```rust
let system_json = std::fs::read_to_string("ieee14.json")?;
tpt_nrg_wasm::validate_system_json(&system_json)?;

let result = tpt_nrg_wasm::run_powerflow_json(&system_json)?;
println!("converged = {} in {} iterations", result.converged, result.iterations);
# Ok::<(), tpt_nrg_wasm::WasmError>(())
```

Microgrid control, with an optional config override:

```rust
let state = r#"{"assets":[
    {"id":"s1","kind":"solar","output_mw":5.0,"capacity_mw":10.0},
    {"id":"b1","kind":"battery","output_mw":0.0,"state_of_charge":0.8,"capacity_mw":5.0},
    {"id":"l1","kind":"load","output_mw":0.0,"capacity_mw":10.0}],
    "grid_connected":true}"#;
let decision = tpt_nrg_wasm::microgrid_step_json(
    state,
    r#"{"demand_mw":8.0,"frequency_hz":59.98}"#,
    Some(r#"{"interval_hours":0.25}"#),
)?;
# Ok::<(), tpt_nrg_wasm::WasmError>(())
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

**Alpha.** The JSON surface and bindings are tested on the native shim, and
CI builds the real `wasm32-unknown-unknown` target; browser demos still
require a browser harness repository.

## Testing

```sh
cargo test -p tpt-nrg-wasm --all-features
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
