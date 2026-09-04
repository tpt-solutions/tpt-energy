# Tutorial: Solar + Battery Microgrid

This tutorial walks through an end-to-end microgrid simulation: a small PV plant charges a battery during the day, and the battery discharges in the evening peak. We use the `tpt-nrg-solar`, `tpt-nrg-battery`, and `tpt-nrg-islanding` crates together.

## Setup

```toml
[dependencies]
tpt-nrg-solar = "0.1"
tpt-nrg-battery = "0.1"
tpt-nrg-islanding = "0.1"
chrono = "0.4"
```

## Step 1 — Site and plant

```rust,no_run
use tpt_nrg_solar::{PvPlant, PvPlantConfig, SolarModel};
use chrono::{TimeZone, Utc};

let site = SolarModel::new(40.0, -105.0, 1600.0, -7.0); // Boulder, CO
let cfg = PvPlantConfig::new(50.0, 30.0, 180.0, 25.0);   // 50 MWp, 30° tilt, south
let pv = PvPlant::new(site, cfg);
```

## Step 2 — 24-hour PV profile

```rust,no_run
use chrono::{Duration, TimeZone, Utc};

let start = Utc.with_ymd_and_hms(2026, 6, 21, 0, 0, 0).unwrap();
let mut hourly_ac_mw = Vec::new();
for h in 0..24 {
    let t = start + Duration::hours(h);
    let out = pv.output_at(t);
    hourly_ac_mw.push(out.ac_power_mw);
}
// hourly_ac_mw now holds 24 MW values; sun-up hours produce 30-40 MW,
// night-time hours produce ~0 MW.
```

## Step 3 — Battery arbitrage on the PV curve

```rust,no_run
use tpt_nrg_battery::{BatteryStorage, DegradationModel};

let mut battery = BatteryStorage::new(100.0, 25.0, 0.9) // 100 MWh, 25 MW, 90% RT
    .with_soc(0.05, 0.20)                                // 5% min, start 20%
    .with_degradation(DegradationModel::li_ion());

for (h, &pv_mw) in hourly_ac_mw.iter().enumerate() {
    let surplus = pv_mw - 30.0;       // local load is 30 MW
    if surplus > 0.0 {
        // Excess PV — charge
        let _ = battery.charge(surplus, 1.0);
    } else if surplus < 0.0 {
        // Deficit — discharge
        let _ = battery.discharge(-surplus, 1.0);
    }
}
```

## Step 4 — Loss-of-mains detection

```rust,no_run
use tpt_nrg_islanding::IslandingDetector;

let detector = IslandingDetector::default();
// If the voltage sags to 0.85 pu (UV trip), islanding is declared.
let islanded = detector.detect_islanding(0.85, 60.0, 0.1);
assert!(islanded);
```

## Full example

The workspace contains a runnable version in `examples/microgrid-islanding.rs`.

## What you've learned

- How to chain resource → storage → protection crates without touching `tpt-nrg-core`.
- The standard "duck curve" pattern: charge during mid-day surplus, discharge at evening peak.
- How IEEE 1547-style islanding detection triggers on voltage, frequency, or RoCoF excursions.
