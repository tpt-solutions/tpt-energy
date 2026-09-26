# tpt-nrg-powerflow

AC and DC power-flow solvers: Newton–Raphson (with generator reactive
power-limit enforcement), Gauss–Seidel, Fast Decoupled, and DC linear
power flow.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- **Newton–Raphson** — full AC solve with damped steps, DC warm start,
  JSON-driven initial voltages, and **PV-to-PQ bus-type switching** at
  generator `Q` limits (Dommel–Tinney outer loop).
- **Gauss–Seidel** — accelerated fixed-point iteration for teaching and
  cross-checking.
- **Fast Decoupled** — Stott & Alsac XB variant with constant B-prime and
  B-double-prime matrices.
- **DC power flow** — linearized `P = B' theta`, lossless, single linear
  solve; also the basis of the NR warm start.
- `PowerFlowResult` with bus voltages/angles, per-branch flows and
  loadings, total losses, and per-generator `P`/`Q` dispatch.
- Validated against the published MATPOWER solutions for **IEEE 14-bus,
  30-bus, and 57-bus** (the 57-bus case converges in 5 iterations once
  Q limits are enforced) — golden fixtures under
  `test-data/golden/powerflow/`.

## Installation

```toml
[dependencies]
tpt-nrg-powerflow = "0.1"
```

## Usage

```rust
use tpt_nrg_core::EnergySystem;
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};

let system = EnergySystem::from_json_file("ieee57.json")?;
let result = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson)
    .with_tolerance(1e-8)
    .with_max_iterations(50)
    .solve(&system)?;

assert!(result.converged);
println!("losses = {:.2} MW in {} iterations",
    result.total_losses_mw, result.iterations);
for (i, v) in result.bus_voltage_magnitude_pu.iter().enumerate() {
    println!("bus {}: {:.4} pu", i + 1, v);
}
# Ok::<(), tpt_nrg_powerflow::PowerFlowError>(())
```

Switch methods via `PowerFlowMethod::{NewtonRaphson, GaussSeidel,
FastDecoupled, DcPowerFlow}`; options (tolerance, iteration cap,
acceleration) via `PowerFlowOptions`.

## Crates.io metadata

- **Categories**: `science`, `simulation`
- **Keywords**: `power-flow`, `newton-raphson`, `gauss-seidel`, `ieee`, `transmission`

## Status

**Stable.** Golden-tested against IEEE 14/30/57-bus with published-anchor
assertions; unit tests cover Q-limit binding, two-bus sanity, and
convergence failure reporting.

## Testing

```sh
cargo test -p tpt-nrg-powerflow --all-features
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
