# API documentation

The Rust API documentation is generated from source via `cargo doc` and
published to the `docs/api/` directory by the `docs.yml` GitHub Actions
workflow on every push to `master`.

To build locally:

```bash
cargo doc --workspace --no-deps --target-dir docs/api/target
```

Then open `docs/api/target/doc/index.html` in a browser.

## Crate index

| Crate | Local doc |
|-------|-----------|
| `tpt-nrg-core` | [`tpt_nrg_core`](tpt_nrg_core/index.html) |
| `tpt-nrg-topology` | [`tpt_nrg_topology`](tpt_nrg_topology/index.html) |
| `tpt-nrg-timeseries` | [`tpt_nrg_timeseries`](tpt_nrg_timeseries/index.html) |
| `tpt-nrg-powerflow` | [`tpt_nrg_powerflow`](tpt_nrg_powerflow/index.html) |
| `tpt-nrg-fault` | [`tpt_nrg_fault`](tpt_nrg_fault/index.html) |
| `tpt-nrg-state-estimation` | [`tpt_nrg_state_estimation`](tpt_nrg_state_estimation/index.html) |
| `tpt-nrg-protection` | [`tpt_nrg_protection`](tpt_nrg_protection/index.html) |
| `tpt-nrg-solar` | [`tpt_nrg_solar`](tpt_nrg_solar/index.html) |
| `tpt-nrg-wind` | [`tpt_nrg_wind`](tpt_nrg_wind/index.html) |
| `tpt-nrg-hydro` | [`tpt_nrg_hydro`](tpt_nrg_hydro/index.html) |
| `tpt-nrg-load` | [`tpt_nrg_load`](tpt_nrg_load/index.html) |
| `tpt-nrg-battery` | [`tpt_nrg_battery`](tpt_nrg_battery/index.html) |
| `tpt-nrg-hydrogen` | [`tpt_nrg_hydrogen`](tpt_nrg_hydrogen/index.html) |
| `tpt-nrg-thermal-storage` | [`tpt_nrg_thermal_storage`](tpt_nrg_thermal_storage/index.html) |
| `tpt-nrg-der` | [`tpt_nrg_der`](tpt_nrg_der/index.html) |
| `tpt-nrg-islanding` | [`tpt_nrg_islanding`](tpt_nrg_islanding/index.html) |
| `tpt-nrg-vpp` | [`tpt_nrg_vpp`](tpt_nrg_vpp/index.html) |
| `tpt-nrg-unit-commitment` | [`tpt_nrg_unit_commitment`](tpt_nrg_unit_commitment/index.html) |
| `tpt-nrg-economic-dispatch` | [`tpt_nrg_economic_dispatch`](tpt_nrg_economic_dispatch/index.html) |
| `tpt-nrg-reserve` | [`tpt_nrg_reserve`](tpt_nrg_reserve/index.html) |
| `tpt-nrg-lcoe` | [`tpt_nrg_lcoe`](tpt_nrg_lcoe/index.html) |
| `tpt-nrg-market` | [`tpt_nrg_market`](tpt_nrg_market/index.html) |
| `tpt-nrg-carbon` | [`tpt_nrg_carbon`](tpt_nrg_carbon/index.html) |
| `tpt-nrg-wasm` | [`tpt_nrg_wasm`](tpt_nrg_wasm/index.html) |

> **Note**: the links above assume the docs have been built. Run
> `cargo doc` once locally (or wait for the CI artifact) before opening.
