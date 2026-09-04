# `tpt-nrg-fault`

Short-circuit (fault) analysis using the method of symmetrical components.

## Fault types

| Variant | Description |
|---------|-------------|
| `ThreePhase` | Three-phase-to-ground. Most severe; sets relay pickups. |
| `LineToLine` | Phase-to-phase. |
| `LineToGround` | Single phase-to-ground. Most common in practice. |
| `DoubleLineToGround` | Two phases to ground. |

## Usage

```rust,no_run
use tpt_nrg_core::EnergySystem;
use tpt_nrg_fault::{FaultAnalyzer, FaultType};

let system = EnergySystem::from_json(include_str!("../../../test-data/ieee/ieee14.json"))?;
let analyzer = FaultAnalyzer::new(&system);
let result = analyzer.calculate_fault_current(FaultType::ThreePhase, /* bus_idx */ 4);
println!("3φ fault at bus 4: I_f = {:.1} pu", result.fault_current_pu);
```

## Sequence networks

The crate uses scalar Thévenin equivalents for each sequence:

- `Z1` — positive sequence (≈ branch impedance).
- `Z2 = Z1` — negative sequence (default for transmission lines).
- `Z0 = 3 * Z1` — zero sequence (default; override per-bus for grounded systems).

These defaults are conservative for HV transmission. For distribution feeders, override `Z0` and `Z2` based on conductor geometry and grounding.

## Golden validation

`test-data/golden/fault/ieee14-bus4.json` carries the expected symmetrical fault current for a 3φ fault at bus 4 of IEEE 14-bus; the unit tests assert the calculator matches within 1%.
