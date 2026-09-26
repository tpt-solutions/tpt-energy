# tpt-nrg-thermal-storage

Thermal energy storage with state-of-charge tracking, SoC bounds, and
round-trip efficiency.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `ThermalStorage` — thermal capacity (`MWh_th`), power rating, and
  round-trip efficiency.
- `charge()` / `discharge()` — state updates with efficiency and limits.
- `with_soc_bounds()` — configurable minimum SoC and initial state.

## Installation

```toml
[dependencies]
tpt-nrg-thermal-storage = "0.1"
```

## Usage

```rust
use tpt_nrg_thermal_storage::ThermalStorage;

let mut store = ThermalStorage::new(200.0, 40.0, 0.95) // 200 MWh_th, 40 MW
    .with_soc_bounds(0.05, 0.5);

let charged = store.charge(40.0, 2.0);
let heat = store.discharge(20.0, 3.0);
println!("SoC now {:.2}", store.soc);
```

## Crates.io metadata

- **Categories**: `science`, `simulation`
- **Keywords**: `thermal-storage`, `storage`, `heat`, `energy`, `simulation`

## Status

**Stable.** Unit-tested for state tracking, bounds, and efficiency.

## Testing

```sh
cargo test -p tpt-nrg-thermal-storage
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
