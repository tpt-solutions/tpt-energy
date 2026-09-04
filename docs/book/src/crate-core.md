# `tpt-nrg-core`

The data-model crate. Everything in TPT Energy reads an `EnergySystem`; nothing here reads from any other crate.

## Key types

| Type | Role |
|------|------|
| `EnergySystem` | Top-level container: id, name, buses, branches, generators, loads, storage, base MVA, frequency. |
| `Bus` | Network node with `BusType` ∈ {Slack, Pv, Pq, Isolated}, voltage setpoint, and shunt admittance. |
| `Branch` | Transmission line: r, x, b pu; tap ratio; phase shift; MVA rating; in-service flag. |
| `Generator` | Connected to a bus; `GeneratorType` ∈ {Thermal, Hydro, Wind, Solar, Nuclear, Geothermal}; P/Q schedule, voltage setpoint, cost curve. |
| `Load` | P/Q demand at a bus. |
| `Storage` | Battery / H₂ / thermal asset tied to a bus. |
| `CostCurve` | Piecewise-linear cost (`$` vs `MW`). |

## Example

```rust,no_run
use tpt_nrg_core::{EnergySystem, Bus, BusType, Branch, Generator, GeneratorType, CostCurve};

let mut sys = EnergySystem::new("toy", "Two-bus toy", 100.0, 60.0);
sys.add_bus(Bus::new(1, "Slack", BusType::Slack).with_voltage_pu(1.06, 0.0))?;
sys.add_bus(Bus::new(2, "Load", BusType::Pq))?;
sys.add_branch(Branch::new(1, "L12", 1, 2, 0.01, 0.1).with_rating_mva(200.0))?;
sys.add_generator(
    Generator::new(1, "G1", GeneratorType::Thermal, 100.0, 20.0)
        .at_bus(1)
        .with_cost_curve(CostCurve::piecewise(20.0, 100.0, 400.0, 2400.0))
)?;
sys.add_load(tpt_nrg_core::Load::new(1, "L1", 2).with_p_q_mw(80.0, 30.0))?;

// Serialize
let json = sys.to_json()?;
```

## JSON (de)serialisation

`EnergySystem::from_json(&str)` and `to_json()` use a simple, schema-stable JSON layout (see `test-data/ieee/ieee14.json` for an example). The schema is deliberately compatible with MATPOWER's `.m` case format so MATPOWER cases can be converted to JSON by an offline converter if desired.

## Errors

The crate uses `thiserror`-derived errors for validation failures (duplicate IDs, missing slack bus, etc.).
