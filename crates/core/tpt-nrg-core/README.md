# tpt-nrg-core

Core data model for TPT Energy: the `EnergySystem` container and the
`Bus`, `Branch`, `Generator`, `Load`, and `Storage` types that make up a
power-system model, plus cost/heat-rate/power curves and JSON
(de)serialization.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy) — power
systems, resource modeling, storage, dispatch, and grid economics in pure
Rust.

## Features

- `EnergySystem`: a validated container for buses, branches, generators,
  loads, storage, and free-form metadata.
- Bus types matching the classical power-flow formulation: `Slack`, `PV`,
  `PQ`, `Isolated`.
- Branch model with per-unit resistance/reactance/susceptance, off-nominal
  tap ratio, and phase shift.
- Generators with `P`/`Q` limits, schedules, voltage setpoints, and
  optional `CostCurve`, `HeatRateCurve`, and `PowerCurve` attachments.
- Full JSON round-trip: `from_json` / `to_json_pretty`, with permissive
  defaults so hand-written test cases stay short.
- Structural validation: reference integrity, duplicate-id rejection, and
  exactly-one-slack enforcement.

## Installation

```toml
[dependencies]
tpt-nrg-core = "0.1"
```

## Usage

```rust
use tpt_nrg_core::{Branch, Bus, BusType, EnergySystem, Generator, GeneratorType};

let mut sys = EnergySystem::new("demo", "Two-bus demo", 100.0, 60.0);
sys.add_bus(Bus::new(1, "Slack", BusType::Slack).with_voltage_pu(1.06, 0.0))?;
sys.add_bus(Bus::new(2, "Load", BusType::Pq).with_load(50.0, 20.0))?;
sys.add_branch(Branch::new(1, "L12", 1, 2, 0.01, 0.05))?;
sys.add_generator(
    Generator::new(1, "G1", GeneratorType::Thermal, 100.0, 10.0)
        .at_bus(1)
        .with_voltage_setpoint(1.06),
)?;
sys.validate()?; // references resolve, exactly one slack bus

let json = sys.to_json_pretty()?;
let back = EnergySystem::from_json(&json)?;
# Ok::<(), tpt_nrg_core::CoreError>(())
```

## Crates.io metadata

- **Categories**: `science`, `simulation`
- **Keywords**: `power-systems`, `energy`, `data-model`, `ieee`, `grid`

## Status

**Stable.** Unit tests cover construction, validation, and JSON
round-trips; every other `tpt-nrg-*` crate builds on this data model.

## Testing

```sh
cargo test -p tpt-nrg-core
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
