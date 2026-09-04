# `tpt-nrg-solar`

Solar position, clear-sky irradiance, plane-of-array transposition, and PV plant output.

## Algorithms

- **Solar position**: NREL SPA-equivalent (Meeus/NOAA-style low-precision algorithm). Sub-degree accuracy; full ±0.0003° SPA is available via the `tpt-sci-astro` substrate.
- **Clear sky**: Ineichen model with Linke turbidity.
- **POA transposition**: isotropic-sky (Liu & Jordan).
- **PV output**: linear DC derating with NOCT cell-temperature model; inverter clipping.

## Example — clear-sky 24-hour profile

```rust,no_run
use tpt_nrg_solar::{PvPlant, PvPlantConfig, SolarModel};
use chrono::{Duration, TimeZone, Utc};

let site = SolarModel::new(40.0, -105.0, 1600.0, -7.0); // Boulder, CO
let pv = PvPlant::new(site, PvPlantConfig::new(50.0, 30.0, 180.0, 25.0));

let start = Utc.with_ymd_and_hms(2026, 6, 21, 0, 0, 0).unwrap();
for h in 0..24 {
    let t = start + Duration::hours(h);
    let out = pv.output_at(t);
    println!("{:02}:00  AC={:6.2} MW  cell_t={:5.1}°C  POA={:6.0} W/m²",
        h, out.ac_power_mw, out.cell_temperature_c, out.poa_w_per_m2);
}
```

## Golden data

- `test-data/golden/solar/nrel-spa-zenith.json` — five reference solar-position samples at known UTC instants.
- `test-data/golden/solar/pv-output-derating.json` — PV output samples including clipping edge cases.

## Earth–Sun distance correction

When the `substrate` feature is enabled, the crate uses `tpt-sci-astro` for a more accurate Earth–Sun distance (affecting the extraterrestrial irradiance by ±3.3% over the year). The self-contained default uses a cosine approximation good to ~1%.
