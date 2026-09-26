# tpt-nrg-solar

Solar resource modeling: sun position, clear-sky irradiance,
plane-of-array transposition, and PV plant output with temperature
derating, soiling losses, and inverter clipping.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `SolarModel::solar_position()` — SPA-equivalent sun position
  (Meeus/NOAA-style algorithms, about ±0.01°, far beyond PV
  resource-assessment needs) with declination, hour angle,
  refraction-corrected apparent altitude, azimuth, and Kasten–Young air
  mass.
- `clear_sky_irradiance()` — Ineichen clear-sky GHI / DNI / DHI with
  altitude-corrected optical depth and Linke turbidity.
- `plane_of_array_irradiance()` — isotropic-sky (Liu & Jordan)
  transposition to tilted surfaces.
- `PvPlant` — NOCT cell-temperature model, linear temperature derating,
  soiling losses, DC/AC ratio clipping, and inverter efficiency.
- Golden validation (`test-data/golden/solar/`) for sun position and PV
  derating.
- The `substrate` feature validates the Earth–Sun distance handling
  against `tpt-sci-astro` orbital mechanics.

## Installation

```toml
[dependencies]
tpt-nrg-solar = "0.1"
```

## Usage

```rust
use chrono::TimeZone;
use tpt_nrg_solar::{PvPlant, PvPlantConfig, SolarModel};

// Site: Boulder, CO at 1600 m.
let site = SolarModel::new(40.0, -105.0, 1600.0, -7.0);
let pos = site.solar_position(chrono::Utc.with_ymd_and_hms(2026, 6, 21, 19, 0, 0).unwrap());
println!("zenith = {:.2} deg, air mass = {:.2}", pos.zenith_deg, pos.air_mass);

// 100 MWp plant, 30 deg tilt, south-facing, 25 C ambient.
let config = PvPlantConfig::new(100.0, 30.0, 180.0, 25.0);
let plant = PvPlant::new(site, config);
let out = plant.output_at(chrono::Utc.with_ymd_and_hms(2026, 6, 21, 19, 0, 0).unwrap());
println!("AC = {:.1} MW (clipped {:.2} MW)", out.ac_power_mw, out.inverter_clipping_mw);
```

## Crates.io metadata

- **Categories**: `science`, `simulation`
- **Keywords**: `solar`, `photovoltaic`, `irradiance`, `solar-position`, `pv`

## Status

**Stable.** Golden-tested for sun position and PV derating behavior; unit
tests cover near-zenith azimuth stability, polar night, and inverter
clipping.

## Testing

```sh
cargo test -p tpt-nrg-solar
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
