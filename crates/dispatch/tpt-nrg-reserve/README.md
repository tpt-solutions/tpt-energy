# tpt-nrg-reserve

Spinning and contingency reserve calculations: headroom accounting,
NERC-style N-1 requirements, and a combined adequacy assessment.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `spinning_reserve_margin()` — headroom on online units
  (`online - output`, floored at 0). Pure availability: it does not
  subtract load, because headroom already nets out the load being served.
- `contingency_reserve_requirement()` — largest-online-unit N-1 plus a
  load fraction (NERC-style).
- `assess_reserves()` — combines both into a `ReserveAssessment` whose
  `margin_mw` is negative when the system is reserve-deficient.

## Installation

```toml
[dependencies]
tpt-nrg-reserve = "0.1"
```

## Usage

```rust
use tpt_nrg_reserve::assess_reserves;

let a = assess_reserves(
    200.0,  // online capacity, MW
    150.0,  // current output, MW
    100.0,  // largest online unit, MW
    500.0,  // load, MW
    0.03,   // load-fraction requirement
);

if a.margin_mw < 0.0 {
    println!("reserve deficit: {:.0} MW", -a.margin_mw);
}
```

## Crates.io metadata

- **Categories**: `science`, `finance`
- **Keywords**: `reserve`, `spinning-reserve`, `nerc`, `contingency`, `power-systems`

## Status

**Stable.** Unit-tested for headroom clamping, the N-1 + 3% requirement,
and deficit flagging.

## Testing

```sh
cargo test -p tpt-nrg-reserve
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
