# `tpt-nrg-wind`

Wind power modelling: vertical wind-speed extrapolation, Weibull probability, and wake-loss models.

## Vertical extrapolation

```rust,no_run
use tpt_nrg_wind::WindModel;

let m = WindModel::new(100.0, 0.03, 10.0); // hub 100 m, z0 = 0.03 m, ref 10 m
let v_at_hub = m.wind_speed_at_height(5.0); // 5 m/s @ 10 m
```

Two profiles:

- **Log profile**: `v(z) = v_ref · ln(z/z0) / ln(z_ref/z0)` — preferred when `z0` is known.
- **Power law**: `v(z) = v_ref · (z/z_ref)^α` — fallback for sites with empirical α.

## Weibull PDF

```rust,no_run
use tpt_nrg_wind::WindModel;

let m = WindModel::new(80.0, 0.03, 10.0);
let f = m.weibull_probability(8.0, 2.0, 8.0); // f(8 m/s | k=2, c=8)
```

Verified to integrate to 1.0 over `[0, ∞)` to within 1e-3.

## Wake models

| Model | Use |
|-------|-----|
| `JensenPark` | Top-hat wake, linear expansion. Fast; industry default. |
| `Frandsen` | Gaussian profile; better for closely-spaced turbines. |
| `EddyViscosity` | Physics-based; more expensive. |

```rust,no_run
use tpt_nrg_wind::{WakeModel, WindFarm, WindTurbine};

let turbine = WindTurbine::new("Generic 2MW", 80.0, 2.0, 3.0, 12.0, 25.0);
let mut farm = WindFarm::new(WakeModel::JensenPark, 90.0).with_wake_decay(0.05);
farm.push(0.0,    0.0, turbine.clone());
farm.push(400.0,  0.0, turbine.clone()); // 5D downstream
farm.push(800.0,  0.0, turbine.clone());

let v_eff = farm.effective_wind_speeds(10.0);  // m/s per turbine
let p_farm = farm.total_power_output(12.0);   // MW
```

## Power curves

`WindTurbine::power_at(v)` uses an optional lookup table if provided; otherwise a cubic fit between cut-in and rated wind speed, with flat rated power to cut-out.
