# `tpt-nrg-topology`

Graph algorithms on the network. Wraps the bus/branch data into a sparse adjacency representation and provides:

- `find_islands()` — connected components.
- `find_shortest_path(from, to)` — Dijkstra on the branch impedance as edge weight.
- `calculate_admittance_matrix()` — Y-bus builder from branch data.

## Y-bus construction

```rust,no_run
use tpt_nrg_core::EnergySystem;
use tpt_nrg_topology::NetworkTopology;

let system = EnergySystem::from_json(include_str!("../../../test-data/ieee/ieee14.json"))?;
let topo = NetworkTopology::from_system(&system);
let y_bus = topo.calculate_admittance_matrix();
println!("Y-bus is {}x{}", y_bus.n, y_bus.n);
```

The dense Y-bus is the source of truth; the `substrate` feature swaps in a sparse representation from `tpt-math-linalg-sparse` when enabled.

## Island detection

```rust,no_run
use tpt_nrg_core::EnergySystem;
use tpt_nrg_topology::NetworkTopology;

let system = EnergySystem::from_json(/* ... */)?;
let topo = NetworkTopology::from_system(&system);
let islands = topo.find_islands();
for (i, island) in islands.iter().enumerate() {
    println!("island {}: {} buses", i, island.len());
}
```

Used by the islanding crate to detect split-system conditions.
