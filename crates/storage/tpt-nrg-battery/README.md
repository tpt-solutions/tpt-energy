# tpt-nrg-battery

Battery storage with state-of-charge tracking, round-trip efficiency
split across the charge/discharge legs, and a capacity-fade degradation
model.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `BatteryStorage` — energy/power ratings, minimum SoC, round-trip
  efficiency, cumulative throughput, and age tracking.
- `charge()` / `discharge()` — SoC updates with efficiency, power, and
  headroom limits; errors instead of silent clamping when a request would
  violate the SoC window.
- `DegradationModel` — cycle life, calendar life, depth-of-discharge
  curve, and temperature derating; `calculate_degradation()` returns the
  capacity-fade fraction.
- `capacity_factor()` — usable capacity fraction after degradation
  (clamped to a 50% floor).
- The `substrate` feature cross-validates cell chemistry assumptions via
  the NMC-811 specific-capacity lookup in `tpt-eng-materials`.

## Installation

```toml
[dependencies]
tpt-nrg-battery = "0.1"
```

## Usage

```rust
use tpt_nrg_battery::{BatteryStorage, DegradationModel};

let mut battery = BatteryStorage::new(100.0, 50.0, 0.9) // 100 MWh, 50 MW, 90% RTE
    .with_soc(0.1, 0.5)
    .with_degradation(DegradationModel::li_ion());

let stored = battery.charge(50.0, 1.0)?;   // MWh stored (after losses)
let out = battery.discharge(50.0, 1.0)?;   // MWh delivered to the grid
println!("SoC = {:.2}, fade factor = {:.3}",
    battery.soc, battery.capacity_factor());
# Ok::<(), tpt_nrg_battery::BatteryError>(())
```

## Crates.io metadata

- **Categories**: `science`, `simulation`
- **Keywords**: `battery`, `state-of-charge`, `li-ion`, `degradation`, `storage`

## Status

**Stable.** Golden-tested SoC cycling fixture
(`test-data/golden/storage/battery-soc-cycling.json`) plus unit tests for
round-trip efficiency, SoC clamping, and error cases.

## Testing

```sh
cargo test -p tpt-nrg-battery --all-features
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
