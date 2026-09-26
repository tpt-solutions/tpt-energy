# tpt-nrg-carbon

Carbon intensity of electricity generation: kg CO2 per MWh from the
generator mix, with lifecycle emission factors per technology.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `emission_factor_kg_per_mwh()` — lifecycle midpoint factors per
  `GeneratorType` (thermal 900, hydro 4, wind 11, solar 45, nuclear 12,
  geothermal 38, other 500).
- `carbon_intensity()` — output-weighted average intensity for a system
  snapshot.
- `total_emissions_tonnes()` — tonnes CO2 over a duration.

## Installation

```toml
[dependencies]
tpt-nrg-carbon = "0.1"
```

## Usage

```rust
use tpt_nrg_carbon::{carbon_intensity, total_emissions_tonnes};

let system = tpt_nrg_core::EnergySystem::from_json_file("system.json")?;
println!("intensity: {:.0} kg CO2/MWh", carbon_intensity(&system));
println!("per day: {:.0} t CO2", total_emissions_tonnes(&system, 24.0));
# Ok::<(), tpt_nrg_core::CoreError>(())
```

## Crates.io metadata

- **Categories**: `science`
- **Keywords**: `carbon`, `emissions`, `co2`, `intensity`, `energy`

## Status

**Stable.** Unit-tested for weighted averaging and emission totals.

## Testing

```sh
cargo test -p tpt-nrg-carbon
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
