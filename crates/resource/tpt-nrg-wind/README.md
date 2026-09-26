# tpt-nrg-wind

Wind resource modeling: vertical wind-speed extrapolation, Weibull
probability, three wake-loss models, and farm-level power output.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `WindModel` — log and power-law vertical profiles for hub-height
  extrapolation; Weibull PDF; Weibull-mean power via 32-point
  Gauss–Legendre quadrature over the turbine power curve.
- Three wake models behind one API (`WindFarm::effective_wind_speeds`):
  - **Jensen/PARK** — top-hat wake, linear expansion, full deficit inside
    the cone.
  - **Frandsen** — rotor-equivalent wake source with a two-zone
    (near/far) deficit and partial-rotor overlap weighting; better for
    closely spaced turbines.
  - **EddyViscosity** — explicit cylindrical diffusion march with a
    downstream-growing eddy viscosity; smooth, radially spreading wake.
- Per-turbine thrust coefficient `C_T` threaded through every deficit
  calculation.
- Power curves: table lookup or cubic fit between cut-in and rated speed.
- Kinematic (multiplicative) deficit superposition across all waking
  pairs.
- Golden fixtures for all three models under `test-data/golden/wind/`.
- The `substrate` feature cross-validates distribution sampling against
  `tpt-math-prob-dist`.

## Installation

```toml
[dependencies]
tpt-nrg-wind = "0.1"
```

## Usage

```rust
use tpt_nrg_wind::{WakeModel, WindFarm, WindTurbine};

// Wind blowing east (90 deg), onshore decay k = 0.05.
let mut farm = WindFarm::new(WakeModel::JensenPark, 90.0).with_wake_decay(0.05);
let turbine = WindTurbine::new("Generic 2MW", 80.0, 2.0, 3.0, 12.0, 25.0)
    .with_thrust_coefficient(0.8);
farm.push(0.0, 0.0, turbine.clone());
farm.push(400.0, 0.0, turbine); // 5 rotor diameters downstream

let v_eff = farm.effective_wind_speeds(10.0);
assert!(v_eff[1] < v_eff[0], "the downstream turbine is waked");
println!("farm output = {:.2} MW", farm.total_power_output(10.0));
```

Switching models is one line — `WakeModel::Frandsen` or
`WakeModel::EddyViscosity` — with identical semantics and golden-tested
behavior.

## Crates.io metadata

- **Categories**: `science`, `simulation`
- **Keywords**: `wind`, `wind-farm`, `wake-model`, `weibull`, `wind-power`

## Status

**Stable.** The Weibull PDF integrates to 1.0 (verified); each wake model
is golden-tested for deficit versus downstream distance.

## Testing

```sh
cargo test -p tpt-nrg-wind --all-features
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
