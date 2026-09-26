# {{project-name}}

An energy-system study built with [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Run it

```sh
cargo run
```

The program loads `system.json`, solves a Newton-Raphson power flow, and
prints the converged voltage profile and total losses.

## Use your own case

The bundled three-bus case is a placeholder. To study a real network, convert
it to the native JSON with the `tpt-nrg` CLI and drop it in:

```sh
tpt-nrg convert case14.m -o system.json
cargo run
```

`case14.m` is any MATPOWER case; the CLI reads and writes MATPOWER, PSS/E, CIM
(IEC 61970), YAML, CSV, and the native JSON.

## What is in here

- `src/main.rs` — the study: load a case, solve it, report the result.
- `system.json` — the case being studied.
- `Cargo.toml` — dependencies on `tpt-nrg-core` (the data model) and
  `tpt-nrg-powerflow` (the solver). Add `tpt-nrg-economic-dispatch`,
  `tpt-nrg-lcoe`, `tpt-nrg-viz`, or any other crate in the
  [crate table](https://github.com/tpt-solutions/tpt-energy#crate-status) as
  the study grows.

## License

Dual licensed under MIT or Apache-2.0, matching the TPT Energy workspace.
