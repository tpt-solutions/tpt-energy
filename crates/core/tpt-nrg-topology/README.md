# tpt-nrg-topology

Network topology utilities for TPT Energy: graph construction from an
`EnergySystem`, connected-component (island) detection, unweighted
shortest paths, and Y-bus admittance matrix construction for lines and
off-nominal-tap transformers.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `NetworkTopology::from_energy_system` — dense-index adjacency list keyed
  by bus id (non-contiguous id schemes supported).
- `find_islands()` — connected-component analysis for island detection.
- `find_shortest_path()` — BFS shortest path returning the bus *and*
  branch sequence.
- `AdmittanceMatrixBuilder` — complex Y-bus (G + jB) with per-unit line
  models, transformer taps, and phase shifters; dense storage by default
  with a sparse COO export (`build_sparse_coo`).
- The `substrate` feature swaps in the `tpt-math-linalg-sparse` sparse
  path behind the same API.

## Installation

```toml
[dependencies]
tpt-nrg-topology = "0.1"
```

## Usage

```rust
use tpt_nrg_topology::NetworkTopology;

let sys = tpt_nrg_core::EnergySystem::from_json_file("ieee14.json")?;
let topo = NetworkTopology::from_energy_system(&sys);

let islands = topo.find_islands();
assert_eq!(islands.len(), 1, "the 14-bus case is one island");

if let Some(path) = topo.find_shortest_path(1, 14) {
    println!("hops: {:?}", path.branches);
}
# Ok::<(), tpt_nrg_core::CoreError>(())
```

Building the admittance matrix:

```rust
use tpt_nrg_topology::AdmittanceMatrixBuilder;

let y_bus = AdmittanceMatrixBuilder::new(&sys).build();
let b_12 = y_bus.b_ij(0, 1); // imaginary part of Y(bus0, bus1), per-unit
```

## Crates.io metadata

- **Categories**: `science`, `algorithms`
- **Keywords**: `power-systems`, `graph`, `ybus`, `network`, `topology`

## Status

**Stable.** Unit-tested for topology construction, island detection, and
shortest paths; the Y-bus builder is exercised by every power-flow golden
test.

## Testing

```sh
cargo test -p tpt-nrg-topology
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
