# tpt-nrg-hydro

Hydroelectric power calculation: net head × turbine flow × efficiency,
with minimum-flow shutdown semantics.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- Standard power formula `P [W] = rho * g * Q * H * eta` (rho = 1000
  kg/m3, g = 9.81 m/s2) converted to MW.
- Flow clamped to the turbine maximum.
- Below the minimum turbine flow the unit produces nothing — a shut-down
  (or environmentally-spilling) unit cannot fabricate power.
- Integrates with `GeneratorType::Hydro` in `tpt-nrg-core`.

## Installation

```toml
[dependencies]
tpt-nrg-hydro = "0.1"
```

## Usage

```rust
use tpt_nrg_hydro::HydroPlant;

// 50 m net head, 90% electromechanical efficiency, 100 m3/s max flow.
let mut plant = HydroPlant::new(50.0, 0.90, 100.0);
plant.min_flow_m3s = 5.0;

assert!((plant.power_output_mw(100.0) - 44.145).abs() < 0.01);
assert!(plant.power_output_mw(3.0).abs() < 1e-12); // below min flow: off
```

## Crates.io metadata

- **Categories**: `science`, `simulation`
- **Keywords**: `hydro`, `hydropower`, `turbine`, `renewables`, `energy`

## Status

**Stable.** Unit-tested against hand-computed reference values.

## Testing

```sh
cargo test -p tpt-nrg-hydro
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
