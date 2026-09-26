# tpt-nrg-cli

The `tpt-nrg` command-line interface: solve a case, convert it between
formats, or draw it — without writing any Rust.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy) — power
systems, resource modeling, storage, dispatch, and grid economics in pure
Rust.

## Features

- `run` — Newton–Raphson, Gauss–Seidel, fast-decoupled, or DC power flow,
  plus economic dispatch, unit commitment, short circuit, and LCOE.
- `convert` — any of MATPOWER, PSS/E, CIM, YAML, CSV, or JSON, in any
  direction.
- `viz` — an SVG single-line diagram with a voltage and loading heatmap.
- `table` output for a terminal, `json` for a script.
- Exit codes that distinguish a bad input from a failed analysis: `0`
  success, `1` the analysis produced no result, `2` bad input or usage.
- Prebuilt binaries for Linux (x86-64, aarch64), macOS (aarch64), and
  Windows (x86-64) on every GitHub release.

## Installation

```sh
cargo install tpt-nrg-cli
```

## Usage

```sh
# Power flow, as a table
tpt-nrg run --system test-data/ieee/ieee14.json --method newton-raphson

# Machine-readable
tpt-nrg run --system case.json --format json

# Economic dispatch at a given load
tpt-nrg run --system case.json --dispatch 100

# 24-hour unit commitment
tpt-nrg run --system case.json --commit

# Three-phase short circuit at a bus
tpt-nrg run --system case.json --fault-bus 4

# Levelized cost of energy
tpt-nrg run --system case.json \
  --lcoe-capex 5e8 --annual-energy-mwh 500000 --discount-rate 0.07

# Convert a MATPOWER case to JSON
tpt-nrg convert --from matpower --to json case14.m -o case14.json

# Draw the case
tpt-nrg viz --system case.json -o diagram.svg
```

`--help` on any subcommand lists every flag.

## Crates.io metadata

- **Categories**: `science`, `command-line-utilities`, `simulation`
- **Keywords**: `power-systems`, `cli`, `matpower`, `power-flow`, `energy`

## Status

**Alpha.** Every subcommand is exercised in CI against the committed IEEE
14-bus case, and the flag set may still grow.

## Testing

```sh
cargo test -p tpt-nrg-cli
```

The CI `cli` job additionally runs every subcommand end to end and asserts
that an unparseable system exits non-zero.

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
