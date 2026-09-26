# tpt-nrg-islanding

Loss-of-mains detection, controlled transition to islanded operation,
and resynchronization with the main grid.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy).

## Features

- `IslandingDetector` — voltage / frequency / RoCoF thresholds per IEEE
  1547 (category II defaults), with a configurable detection time window.
- `transition_to_island()` — balanced transition planning: load shedding,
  storage dispatch, and the new V/f reference for the island.
- `resynchronize()` — phase / frequency / voltage matching before
  reclosing, with correct 2-pi phase wrapping.
- Plain-function API over `f64` inputs — easy to drive from real-time
  loops or scenario tests.

## Installation

```toml
[dependencies]
tpt-nrg-islanding = "0.1"
```

## Usage

```rust
use tpt_nrg_islanding::{resynchronize, transition_to_island, IslandingDetector};

let detector = IslandingDetector::new();
let lost_mains = detector.detect_islanding(0.95, 60.02, 1.2); // V, f, RoCoF

if lost_mains {
    // Island: 10 MW load, 8 MW local generation, 3 MW of storage.
    let t = transition_to_island(10.0, 8.0, 3.0, 1.0, 60.0);
    println!("shed {:.1} MW, storage {:.1} MW", t.load_shed_mw, t.storage_dispatch_mw);
}

let sync = resynchronize(1.0, 1.0, 0.0, 0.0, 59.99, 60.0, 0.05, 0.05, 0.1);
if sync.success {
    println!("safe to reclose");
}
```

## Crates.io metadata

- **Categories**: `science`, `simulation`
- **Keywords**: `islanding`, `microgrid`, `rocof`, `ieee-1547`, `resynchronization`

## Status

**Stable.** Unit tests cover each detection mechanism, surplus/deficit
transitions, storage coverage, and phase-wrap resynchronization.

## Testing

```sh
cargo test -p tpt-nrg-islanding
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
