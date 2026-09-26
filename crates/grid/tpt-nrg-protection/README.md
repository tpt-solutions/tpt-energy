# tpt-nrg-protection

Protection coordination modeling: IEC 60255-151 inverse-time relay
curves with pickup, time-multiplier, and coordination-margin checks.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `RelayCurve` — IEC 60255-151 characteristics: standard inverse, very
  inverse, extremely inverse, and definite time.
- `Relay` — pickup current, time-multiplier setting (TMS), curve type,
  and coordinate time delay.
- `check_coordination()` — verifies that a backup relay trips after the
  primary relay with the required margin (typically 0.2–0.4 s) across a
  range of fault currents.

## Installation

```toml
[dependencies]
tpt-nrg-protection = "0.1"
```

## Usage

```rust
use tpt_nrg_protection::{check_coordination, Relay, RelayCurve};

let primary = Relay {
    id: 1, branch_id: 7, end: tpt_nrg_protection::RelayEnd::From,
    pickup_pu: 1.2, time_multiplier: 0.1,
    curve: RelayCurve::StandardInverse, time_delay_s: 0.0,
};
let backup = Relay { time_multiplier: 0.3, time_delay_s: 0.25, ..primary.clone() };

let fault_currents = vec![2.0, 3.0, 5.0, 8.0];
assert!(check_coordination(&primary, &backup, &fault_currents));
```

## Crates.io metadata

- **Categories**: `science`, `simulation`
- **Keywords**: `relay`, `protection`, `iec-60255`, `coordination`, `power-systems`

## Status

**Stable.** Unit-tested for curve trip times and coordination scenarios.

## Testing

```sh
cargo test -p tpt-nrg-protection
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
