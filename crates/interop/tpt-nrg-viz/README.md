# tpt-nrg-viz

Dependency-free SVG rendering for TPT Energy power-flow results: a
single-line diagram with a bus voltage heatmap and a branch loading
heatmap.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy) — power
systems, resource modeling, storage, dispatch, and grid economics in pure
Rust.

## Features

- One `String` out: no asset pipeline, no JavaScript, no template engine.
- A bus voltage heatmap on the planning limits (±5% of nominal, widened to
  ±10%).
- A branch loading heatmap on thermal rating, with a *separate* scale from
  the voltage one — a lightly loaded line is healthy, so the two cannot share
  a mirror-image ramp.
- Deterministic, diffable layout: buses are placed by shortest-path depth
  from the slack bus, so the diagram reads left to right and the same input
  always produces the same bytes.
- Works with or without a solved result.
- Accessible from the CLI (`tpt-nrg viz`), the Python bindings
  (`System.to_svg()`), and the WASM bindings (`wasm_visualize_json`).

## Installation

```toml
[dependencies]
tpt-nrg-viz = "0.1"
```

## Usage

```rust
use tpt_nrg_core::EnergySystem;
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};
use tpt_nrg_viz::{render, VizOptions};

let system = EnergySystem::from_json(include_str!("../../../test-data/ieee/ieee14.json"))?;
let result = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson).solve(&system)?;

let svg = render(&system, Some(&result), &VizOptions::default());
std::fs::write("diagram.svg", &svg)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

From the command line:

```sh
tpt-nrg viz --system case.json -o diagram.svg
```

An example that renders a committed test case:

```sh
cargo run -p tpt-nrg-viz --example render_ieee14
```

## Options

```rust
use tpt_nrg_viz::VizOptions;

let options = VizOptions {
    title: "Evening peak".to_string(),
    show_legend: false,
    show_flow_labels: false,
    ..VizOptions::default()
};
```

## Crates.io metadata

- **Categories**: `science`, `visualization`, `graphics`
- **Keywords**: `power-systems`, `svg`, `visualization`,
  `single-line-diagram`, `power-flow`

## Status

**Alpha.** The renderer is deterministic and tested, but the layout is a
breadth-first grid rather than a true one-line diagram: it does not attempt
orthogonal routing, label placement to avoid overlap, or interactive
editing.

## Testing

```sh
cargo test -p tpt-nrg-viz
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
