# tpt-nrg-fault

Short-circuit (fault) analysis using the symmetrical-component method:
three-phase, line-to-line, line-to-ground, and double-line-to-ground
faults.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `FaultAnalyzer` over an `EnergySystem` with per-bus Thevenin impedance
  estimation (graph search from the slack bus accumulating series R/X).
- Sequence-network construction (positive / negative / zero) with the
  standard `Z2 = Z1`, `Z0 = 3*Z1` defaults and an explicit
  `SequenceNetwork` override for detailed studies.
- `calculate_fault_current()` for all four classic fault types, returning
  per-sequence currents and the fault-current magnitude.
- Verified against textbook cases (Glover, Sarma & Overbye, *Power System
  Analysis and Design*, Example 7.5).

## Installation

```toml
[dependencies]
tpt-nrg-fault = "0.1"
```

## Usage

```rust
use tpt_nrg_core::EnergySystem;
use tpt_nrg_fault::{FaultAnalyzer, FaultType};

let system = EnergySystem::from_json_file("ieee14.json")?;
let analyzer = FaultAnalyzer::new(&system);

let fault = analyzer.calculate_fault_current(5, FaultType::ThreePhase);
println!("|If| = {:.3} pu at bus 5", fault.i_fault_pu);
println!("sequence currents: {:?}", fault.sequence);
```

## Crates.io metadata

- **Categories**: `science`, `simulation`
- **Keywords**: `short-circuit`, `fault`, `symmetrical-components`, `protection`, `power-systems`

## Status

**Stable.** Unit-tested against published short-circuit reference cases
and IEEE bus data.

## Testing

```sh
cargo test -p tpt-nrg-fault
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
