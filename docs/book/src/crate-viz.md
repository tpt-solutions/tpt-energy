# `tpt-nrg-viz`

Renders a power-flow result as a single SVG document: a single-line diagram
with a bus voltage heatmap and a branch loading heatmap.

There is no asset pipeline, no JavaScript, and no template engine — the
output is a `String` you can write to a file, serve over HTTP, or inline into
a report.

## Usage

```rust
use tpt_nrg_core::EnergySystem;
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};
use tpt_nrg_viz::{render, VizOptions};

let system = EnergySystem::from_json(include_str!("../../test-data/ieee/ieee14.json"))?;
let result = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson).solve(&system)?;

let svg = render(&system, Some(&result), &VizOptions::default());
assert!(svg.starts_with("<svg"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

`result` may be `None`; the buses are then drawn at their scheduled voltages
and the branches carry no flows, which is useful for inspecting a case
before solving it.

From the command line:

```sh
tpt-nrg viz --system test-data/ieee/ieee14.json -o diagram.svg
```

## Options

[`VizOptions`](https://docs.rs/tpt-nrg-viz) controls the overlays:

| Field             | Default        | Effect                                    |
|-------------------|----------------|-------------------------------------------|
| `title`           | system name    | SVG `<title>`; empty means "use the name" |
| `show_bus_ids`    | `true`         | bus id and per-unit voltage labels        |
| `show_flow_labels`| `true`         | per-branch MW labels                      |
| `show_legend`     | `true`         | the two heatmap legends                   |
| `show_generators` | `true`         | capacity marker under generating buses    |
| `background`      | `#ffffff`      | `None` leaves the canvas transparent      |

## The two heatmaps

Bus voltage and branch loading use *different* scales, because they mean
opposite things: a low voltage is a problem, but a lightly loaded line is
healthy.

| Voltage (pu)        | Band            | Colour    |
|---------------------|-----------------|-----------|
| `< 0.95`            | critical low    | `#7f1d1d` |
| `0.95 – 1.00`       | low             | `#dc2626` |
| `1.00 – 1.05`       | normal          | `#16a34a` |
| `1.05 – 1.10`       | high            | `#f59e0b` |
| `> 1.10`            | critical high   | `#b91c1c` |
| no data             | unknown         | `#9ca3af` |

| Loading (of rating) | Band            | Colour    |
|---------------------|-----------------|-----------|
| `0 – 95%`           | normal          | `#16a34a` |
| `95 – 100%`         | high            | `#f59e0b` |
| `100 – 105%`        | overload        | `#dc2626` |
| `> 105%`            | severe overload | `#7f1d1d` |
| no rating           | unknown         | `#9ca3af` |

Both scales are exposed as `Band` and `LoadingBand` so a caller can reuse
them for a legend, a report, or a threshold check.

## Layout

Bus positions are deterministic, which is what makes the output diffable and
the tests meaningful. A general force-directed layout would be overkill for a
one-line diagram and would not be reproducible.

Buses are assigned a depth equal to their shortest path from the slack bus
(after excluding out-of-service branches), sorted by `(depth, id)`, and
filled into a grid column by column. So the slack bus is leftmost and the
network reads left to right, the way a one-line diagram is normally read.
Buses not connected to the reference are placed after them rather than
dropped.

```rust
use tpt_nrg_core::EnergySystem;
use tpt_nrg_viz::layout;

let system = EnergySystem::from_json(include_str!("../../test-data/ieee/ieee14.json"))?;
let l = layout::compute(&system);
assert_eq!(l.positions.len(), system.buses.len());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Rendering an example

```sh
cargo run -p tpt-nrg-viz --example render_ieee14
```

writes `target/ieee14.svg`.

## Testing

```sh
cargo test -p tpt-nrg-viz
```

The suite checks that the output is well-formed and deterministic, that
out-of-service branches are dashed, that the bus fill tracks the voltage
band, that the legend can be disabled, and that titles are XML-escaped.
