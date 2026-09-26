# tpt-nrg-hydrogen

Hydrogen storage: electrolyzer (electricity to H2), fuel cell (H2 to
electricity), and a tank-coupled system with round-trip efficiency.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `Electrolyzer` — specific-energy-based H2 production
  (`kWh/kg` efficiency, MW power rating).
- `FuelCell` — H2-to-electricity conversion with system-level
  specific energy.
- `HydrogenSystem` — tank-coupled charge/discharge with capacity limits
  and combined round-trip efficiency accounting.

## Installation

```toml
[dependencies]
tpt-nrg-hydrogen = "0.1"
```

## Usage

```rust
use tpt_nrg_hydrogen::{Electrolyzer, FuelCell, HydrogenSystem};

// 55 kWh/kg electrolyzer, 20 kWh/kg fuel cell, 50 MW each way.
let system = HydrogenSystem::new(
    Electrolyzer::new(55.0, 50.0),
    FuelCell::new(20.0, 50.0),
    1000.0, // kg tank
);

let kg = system.electrolyzer.produce_hydrogen(100.0); // 100 MWh in
let mwh = system.fuel_cell.generate_electricity(kg);  // back out
println!("100 MWh -> {kg:.1} kg -> {mwh:.1} MWh (RTE {:.0}%)",
    system.round_trip_efficiency() * 100.0);
```

## Crates.io metadata

- **Categories**: `science`, `simulation`
- **Keywords**: `hydrogen`, `electrolyzer`, `fuel-cell`, `storage`, `energy`

## Status

**Stable.** Golden-tested efficiency fixture
(`test-data/golden/storage/hydrogen-efficiency.json`).

## Testing

```sh
cargo test -p tpt-nrg-hydrogen
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
