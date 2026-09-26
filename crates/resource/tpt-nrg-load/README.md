# tpt-nrg-load

Load forecasting and demand-response models: calendar shape,
temperature sensitivity, and price elasticity.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `LoadModel` — base load plus:
  - a 168-entry weekly shape (hour-of-week multipliers),
  - linear temperature sensitivity around a reference temperature,
  - constant-elasticity price response.
- `forecast_load()` — time / weather / calendar-based forecast for a
  given hour-of-week, temperature, and price.
- `demand_response()` — price-elastic load reduction relative to a
  reference price.
- Golden load profiles (residential / commercial / industrial CSVs) under
  `test-data/load-profiles/`.

## Installation

```toml
[dependencies]
tpt-nrg-load = "0.1"
```

## Usage

```rust
use tpt_nrg_load::LoadModel;

let model = LoadModel::new(100.0)            // 100 MW base
    .with_temperature_sensitivity(1.5, 18.0) // +1.5 MW per degC below 18
    .with_price_elasticity(-0.5);

let base = model.forecast_load(9 * 24 + 18, 12.0, 50.0); // Wed 18:00, 12 C, $50
let shaved = model.demand_response(120.0, 50.0);         // high-price response
println!("forecast {base:.1} MW, demand response -{shaved:.2} MW");
```

## Crates.io metadata

- **Categories**: `science`, `simulation`
- **Keywords**: `load-forecasting`, `demand-response`, `electricity`, `energy`, `load-profile`

## Status

**Stable.** Unit-tested for shape application, temperature sensitivity,
and elasticity response.

## Testing

```sh
cargo test -p tpt-nrg-load
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
