# tpt-nrg-lcoe

Energy-project finance: levelized cost of energy (LCOE), net present
value (NPV), and internal rate of return (IRR).

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `levelized_cost_of_energy()` — discounted LCOE from CAPEX, fixed and
  variable O&M, fuel, annual energy, lifetime, and discount rate, with
  the capital-recovery factor handled for you (and a correct annuity
  limit as the discount rate approaches zero).
- `net_present_value()` — discounted cash-flow series.
- `internal_rate_of_return()` — bisection IRR over `[-0.99, 10.0]`,
  returning `None` when the sign change cannot be bracketed.

## Installation

```toml
[dependencies]
tpt-nrg-lcoe = "0.1"
```

## Usage

```rust
use tpt_nrg_lcoe::{internal_rate_of_return, levelized_cost_of_energy, LcoeInputs};

// $1B CAPEX, $10M/yr fixed O&M, 438 GWh/yr, 30 years at 7%.
let inputs = LcoeInputs::new(1.0e9, 1.0e7, 0.0, 0.0, 219_000.0, 30, 0.07, 0.5);
println!("LCOE = {:.0} $/MWh", levelized_cost_of_energy(&inputs));

let cf = vec![-1000.0, 1100.0];
let irr = internal_rate_of_return(&cf).unwrap();
assert!((irr - 0.10).abs() < 1e-3);
```

## Crates.io metadata

- **Categories**: `finance`, `science`
- **Keywords**: `lcoe`, `npv`, `irr`, `energy-economics`, `project-finance`

## Status

**Stable.** Unit-tested against hand-computed finance references (NPV = 0
at the IRR, annuity limits).

## Testing

```sh
cargo test -p tpt-nrg-lcoe
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
