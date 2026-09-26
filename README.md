# tpt-energy

> Power systems, resource modeling, storage, dispatch, and grid economics — pure Rust.

**Org:** TPT Solutions · **License:** MIT OR Apache-2.0 · **Repo:** `tpt-energy`

[![CI](https://github.com/tpt-solutions/tpt-energy/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-energy/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

TPT Energy is a comprehensive, open-source toolkit for power-systems engineering
and energy-systems modeling, written in Rust and built on the TPT substrate
crates (`tpt-math`, `tpt-engineering`, `tpt-science`). It is designed for
research, planning, dispatch, real-time grid control, and in-browser
exploration via WebAssembly.

## Install

> **Status:** no release has been published yet, so nothing is on crates.io,
> PyPI, or npm and this repository carries no version badges for them. The
> commands below are what the release workflow (`release.yml`) will run on the
> first `v*.*.*` tag; until then, install from a checkout, as shown next.

From a checkout, which works today:

```sh
git clone https://github.com/tpt-solutions/tpt-energy
cd tpt-energy
cargo install --path crates/cli/tpt-nrg-cli   # the `tpt-nrg` command
# or run it without installing:
cargo run -p tpt-nrg-cli -- --help
```

As a library, from a checkout:

```toml
[dependencies]
tpt-nrg-core = { path = "crates/core/tpt-nrg-core" }
tpt-nrg-powerflow = { path = "crates/grid/tpt-nrg-powerflow" }
```

Once a release is published, these become the supported forms:

```toml
# crates.io
[dependencies]
tpt-nrg-core = "0.1"
tpt-nrg-powerflow = "0.1"
```

```sh
cargo install tpt-nrg-cli      # command-line interface
pip install tpt-nrg            # Python bindings
npm install tpt-nrg-wasm       # WebAssembly bindings
```

The container image needs no Rust toolchain at all:

```sh
docker run --rm -v "$PWD:/cases" ghcr.io/tpt-solutions/tpt-energy run --system /cases/case.m
```

## Five-minute quickstart (no Rust)

For grid engineers who want an answer rather than a library, the CLI alone is
enough: take a public MATPOWER case, convert it, solve it, look at it.

```sh
# 1. Get a case. The IEEE test cases are public; any `case*.m` works.
curl -LO https://raw.githubusercontent.com/PowerAPI-Validation/Python-MATPOWER/master/powerdata/case14.m

# 2. Convert it to the native JSON (MATPOWER, PSS/E, CIM, YAML, CSV all work).
tpt-nrg convert case14.m -o case14.json

# 3. Solve it. The format is inferred from the file extension.
tpt-nrg run --system case14.json

# 4. Look at it: a single-line diagram with the voltage profile.
tpt-nrg viz --system case14.json -o case14.svg

# 5. Check that the conversion did not quietly lose anything.
tpt-nrg convert case14.m --round-trip
```

Step 5 is the one that catches bad data: it writes the case back out in the
same format, reads it again, and compares the two models field by field. It
exits non-zero and prints the paths that differ. To compare two cases
directly:

```sh
tpt-nrg convert case14.json --diff other-case.m
```

Without the CLI, `tpt-nrg new my-study` scaffolds a Rust project that loads
and solves a case, wired to this checkout:

```sh
tpt-nrg new my-study --local .
cd my-study && cargo run
```

## Browser playground

[`playground/`](playground/) is a static page that loads the WebAssembly
build, solves a case you paste or pick, and draws the voltage profile — no
server, no build step, no install. The `playground.yml` workflow deploys it to
GitHub Pages on every push to `master`; locally, build the package and serve
the directory:

```sh
tools/build-npm-package.sh
cp -r dist/npm playground/pkg
python -m http.server --directory playground
```


## Energy Cycle

TPT Energy follows the **Energy Cycle**: resources → grid → storage → microgrid
→ dispatch → economics, all of which integrate back into the TPT substrate
(materials degradation → device physics → grid dispatch).

```mermaid
flowchart LR
    A[Resource<br>solar · wind · hydro · load] --> B[Grid<br>power flow · fault · state est · protection]
    B --> C[Storage<br>battery · hydrogen · thermal]
    C --> D[Microgrid<br>DER · islanding · VPP]
    D --> E[Dispatch<br>unit commitment · economic · reserves]
    E --> F[Economics<br>LCOE · market · carbon]
    F --> A
```

## Crate Status

| Crate                             | Description                                  | Status   |
|-----------------------------------|----------------------------------------------|----------|
| `tpt-nrg-core`                    | Energy system data model                     | Stable   |
| `tpt-nrg-topology`                | Graph & Y-bus construction                   | Stable   |
| `tpt-nrg-timeseries`              | Load/generation/price profiles               | Stable   |
| `tpt-nrg-wasm`                    | Browser/edge WASM bindings                   | Alpha    |
| `tpt-nrg-solar`                   | Solar position & PV output                   | Stable   |
| `tpt-nrg-wind`                    | Wind power & wake losses                     | Stable   |
| `tpt-nrg-hydro`                   | Hydro power calculation                      | Stable   |
| `tpt-nrg-load`                    | Load forecasting & demand response           | Stable   |
| `tpt-nrg-powerflow`               | AC/DC power flow solvers (NR with Q limits)  | Stable   |
| `tpt-nrg-fault`                   | Short-circuit (fault) analysis               | Stable   |
| `tpt-nrg-state-estimation`        | DC-approx WLS grid state estimator           | Alpha    |
| `tpt-nrg-protection`              | Relay coordination & protection zones        | Stable   |
| `tpt-nrg-battery`                 | Battery storage & degradation                | Stable   |
| `tpt-nrg-hydrogen`                | Electrolyzer / fuel-cell / H2 storage        | Stable   |
| `tpt-nrg-thermal-storage`         | Thermal storage                              | Stable   |
| `tpt-nrg-der`                     | Distributed energy resources                 | Stable   |
| `tpt-nrg-islanding`               | Loss-of-mains detection & resync             | Stable   |
| `tpt-nrg-vpp`                     | Virtual power plant aggregation              | Stable   |
| `tpt-nrg-unit-commitment`         | Priority-list UC (MILP behind `substrate`)   | Alpha    |
| `tpt-nrg-economic-dispatch`       | Economic dispatch & storage arbitrage        | Stable   |
| `tpt-nrg-reserve`                 | Spinning & contingency reserve               | Stable   |
| `tpt-nrg-lcoe`                    | LCOE / NPV / IRR                             | Stable   |
| `tpt-nrg-market`                  | Market signal modeling                       | Stable   |
| `tpt-nrg-carbon`                  | Carbon intensity                             | Stable   |
| `tpt-nrg-interop`                 | MATPOWER / PSS-E / CIM / YAML / CSV I/O      | Alpha    |
| `tpt-nrg-viz`                     | SVG single-line diagram + heatmap            | Alpha    |
| `tpt-nrg-cli` (`tpt-nrg` binary)  | Command-line interface                       | Alpha    |
| `tpt-nrg-python` (`tpt_nrg` module) | Python bindings, built with maturin        | Alpha    |

See [`docs/book/src/crate-status.md`](docs/book/src/crate-status.md) for
detailed maturity notes per crate.

## Per-crate documentation

Every crate ships its own documentation, in its own directory:

- `crates/**/<crate>/README.md` — what the crate does, installation,
  a usage example, crates.io categories/keywords, status, and test
  instructions.
- `crates/**/<crate>/CHANGELOG.md` — per-crate changes in
  [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.
- The [mdBook](docs/book/src/SUMMARY.md) walks through the architecture
  and each crate with tutorials.

## Quick Start

```toml
# Cargo.toml -- from a checkout; use `tpt-nrg-core = "0.1"` once published
[dependencies]
tpt-nrg-core = { path = "crates/core/tpt-nrg-core" }
tpt-nrg-powerflow = { path = "crates/grid/tpt-nrg-powerflow" }
```

```rust
use tpt_nrg_core::EnergySystem;
use tpt_nrg_powerflow::{PowerFlowSolver, PowerFlowMethod};

let system = EnergySystem::from_json(include_str!("ieee14.json"))?;
let solver = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson);
let result = solver.solve(&system)?;
println!("{:?}", result);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Command line

```sh
tpt-nrg run --system test-data/ieee/ieee14.json --method newton-raphson
tpt-nrg run --system case.json --fault-bus 4
tpt-nrg run --system case.json --dispatch 100
tpt-nrg convert case.m -o case.json
tpt-nrg convert case.json --diff case.m          # compare two cases
tpt-nrg convert case.m --round-trip              # check a writer is lossless
tpt-nrg viz --system case.json -o diagram.svg
tpt-nrg new my-study --local .                  # scaffold a project
```

Every subcommand infers the input format from the file extension (`case.m`,
`case.raw`, `case.rdf`, `case.yaml`, `case.csv`, `case.json`) unless `--from`
says otherwise. `convert --diff` and `--round-trip` exit `1` when the two
models differ, so they drop straight into a pipeline:

```sh
tpt-nrg convert case.m --round-trip || echo "the MATPOWER writer lost data"
```

The IEEE golden cases are documented as one-line commands in the
[golden-case gallery](docs/book/src/golden-cases.md).

## Python

```python
import tpt_nrg

system = tpt_nrg.System.from_file("test-data/ieee/ieee14.json")
result = system.power_flow()
print(result.converged, result.losses_mw)

try:
    system.power_flow(max_iterations=1)
except tpt_nrg.EnergyError as exc:
    if tpt_nrg.error_kind(exc) == "non_convergence":
        ...  # loosen the tolerance or fix the case
```

## Interoperability

`MATPOWER`, `PSS/E`, `CIM` (IEC 61970), `YAML`, `CSV`, and the native `JSON`
are all two-way via `tpt-nrg-interop`, and reachable from the CLI, the Python
bindings, and the browser bindings:

```rust
use tpt_nrg_interop::{from_text, to_text, Format};

let system = from_text(matpower_source, Format::Matpower)?;
let cim = to_text(&system, Format::Cim)?;
# Ok::<(), tpt_nrg_interop::InteropError>(())
```

## Visualization

`tpt-nrg-viz` renders a single-line diagram with a voltage and loading
heatmap as plain SVG — no asset pipeline, no JavaScript:

```rust
use tpt_nrg_viz::{render, VizOptions};

let svg = render(&system, Some(&result), &VizOptions::default());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Examples

- `examples/ieee-14-bus-powerflow` — solve the IEEE 14-bus case
- `examples/solar-farm-layout` — solar position + PV output profile
- `examples/wind-farm-wake` — wake models (Jensen / Frandsen / eddy-viscosity)
- `examples/microgrid-islanding` — islanding event + battery dispatch
- `examples/battery-arbitrage` — storage arbitrage from price forecast
- `examples/energy-cycle` — end-to-end: resource → storage → grid →
  dispatch → economics

Two task-shaped worked examples answer complete questions rather than
demonstrating a single API:

- `examples/contingency-analysis` — imports a MATPOWER case, checks the import
  against the committed JSON copy of the same case, then runs an N-1 outage
  sweep and ranks the contingencies that break the planning limits
- `examples/battery-sizing` — sizes a battery for peak shaving from a metered
  load CSV, sizes the energy by simulating the duty cycle, then confirms the
  size against the real `BatteryStorage` SoC model and costs it out

The `examples/` directory is its own cargo workspace; build it with
`cargo check --manifest-path examples/Cargo.toml --all-targets`.

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at
your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
