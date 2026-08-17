# tpt-energy — Project Todo

**Org:** TPT Solutions · **License:** MIT OR Apache-2.0 · **Repo:** `tpt-energy`

---

## Phase 0 — Project Setup & Governance

### Repo root
- [ ] `Cargo.toml` (workspace root, members = all crates below)
- [ ] `LICENSE-MIT`
- [ ] `LICENSE-APACHE`
- [ ] `README.md` (per Section 13 template — crate status table, quick start, Energy Cycle diagram, license section)
- [ ] `CONTRIBUTING.md` (fork → branch → code+tests → fmt/clippy/test → `cargo deny check licenses` → PR w/ DCO sign-off → RFC if needed → 2 approvals)
- [ ] `SECURITY.md` (private disclosure process)
- [ ] `CODE_OF_CONDUCT.md`
- [ ] `CHANGELOG.md`
- [ ] `deny.toml` (allow MIT/Apache-2.0/BSD-2/BSD-3/ISC/Zlib/Unicode-3.0; `copyleft = "deny"`; `unlicensed = "deny"`)
- [ ] `rustfmt.toml`
- [ ] `clippy.toml`

### GitHub scaffolding
- [ ] `.github/workflows/ci.yml`
- [ ] `.github/workflows/license.yml` (runs `cargo deny check licenses`)
- [ ] `.github/workflows/benchmark.yml`
- [ ] `.github/workflows/docs.yml`
- [ ] `.github/workflows/release.yml` (SemVer, 6-week cadence)
- [ ] `.github/ISSUE_TEMPLATE/`
- [ ] `.github/PULL_REQUEST_TEMPLATE.md`
- [ ] Public GitHub Projects board (roadmap)

### Workspace skeleton
- [ ] Create `crates/core/{tpt-nrg-core, tpt-nrg-topology, tpt-nrg-timeseries, tpt-nrg-wasm}` (empty crate stubs)
- [ ] Create `crates/resource/{tpt-nrg-solar, tpt-nrg-wind, tpt-nrg-hydro, tpt-nrg-load}` (empty crate stubs)
- [ ] Create `crates/grid/{tpt-nrg-powerflow, tpt-nrg-fault, tpt-nrg-state-estimation, tpt-nrg-protection}` (empty crate stubs)
- [ ] Create `crates/storage/{tpt-nrg-battery, tpt-nrg-hydrogen, tpt-nrg-thermal-storage}` (empty crate stubs)
- [ ] Create `crates/microgrid/{tpt-nrg-der, tpt-nrg-islanding, tpt-nrg-vpp}` (empty crate stubs)
- [ ] Create `crates/dispatch/{tpt-nrg-unit-commitment, tpt-nrg-economic-dispatch, tpt-nrg-reserve}` (empty crate stubs)
- [ ] Create `crates/economics/{tpt-nrg-lcoe, tpt-nrg-market, tpt-nrg-carbon}` (empty crate stubs)
- [ ] `examples/` dir stubs: `ieee-14-bus-powerflow`, `solar-farm-layout`, `wind-farm-wake`, `microgrid-islanding`, `battery-arbitrage`
- [ ] `test-data/{ieee, nrel, load-profiles, golden}` directories
- [ ] `benches/` stubs: `newton-raphson-100k-bus.rs`, `unit-commitment-milp.rs`, `solar-spa-calculation.rs`
- [ ] `docs/{book, rfc, api}` directories
- [ ] `rfcs/0001-sparse-powerflow.md` (stub)
- [ ] `rfcs/0002-battery-degradation-integration.md` (stub)
- [ ] `rfcs/0003-microgrid-islanding.md` (stub)

### Substrate dependencies
- [ ] Wire up `tpt-math` (`tpt-math-linalg-fixed`, `tpt-math-optimize-general`, `tpt-math-prob-dist`, `tpt-math-signal-filter`) as workspace deps
- [ ] Wire up `tpt-engineering` (thermodynamics, fluid dynamics, material properties) as workspace dep
- [ ] Wire up `tpt-science` (meteorology, astronomy, chemistry) as workspace dep

---

## Phase 1 — Foundation

### `tpt-nrg-core`
- [ ] `EnergySystem` struct (id, name, buses, branches, generators, loads, storage, base_mva, frequency_hz)
- [ ] `Bus` struct + `BusType` enum (Slack, PV, PQ, Isolated)
- [ ] `Branch` struct (resistance/reactance/susceptance pu, tap_ratio, phase_shift, rating_mva)
- [ ] `Generator` struct + `GeneratorType` enum (Thermal, Hydro, Wind, Solar, Nuclear, Geothermal)
- [ ] `CostCurve`, `HeatRateCurve`, `PowerCurve` supporting types
- [ ] JSON (de)serialization for `EnergySystem` (`from_json`)
- [ ] Unit tests for all core struct construction/validation

### `tpt-nrg-topology`
- [ ] `NetworkTopology` struct (adjacency_matrix, bus_map)
- [ ] `find_islands()` — connected components analysis
- [ ] `find_shortest_path()` — Dijkstra's algorithm
- [ ] `calculate_admittance_matrix()` — Y-bus builder from branch data (via `tpt-math-linalg-fixed`)
- [ ] Unit tests: topology construction, island detection, shortest path

### `tpt-nrg-timeseries`
- [ ] Time-series container type(s) for load/generation/price profiles
- [ ] Resampling / interpolation utilities
- [ ] Unit tests for time-series operations

### Phase 1 Milestone
- [ ] Parse IEEE 14-bus test case (`test-data/ieee/`) into `EnergySystem`
- [ ] Build Y-bus admittance matrix from parsed 14-bus case and validate against known values

---

## Phase 2 — Power Flow & Fault Analysis

### `tpt-nrg-powerflow`
- [ ] `PowerFlowSolver` struct + `PowerFlowMethod` enum (NewtonRaphson, GaussSeidel, FastDecoupled, DcPowerFlow)
- [ ] `solve()` dispatch method
- [ ] `newton_raphson()` — Y-bus build, mismatch calc, sparse Jacobian, iterative solve
- [ ] `dc_power_flow()`
- [ ] `PowerFlowResult` + `BranchFlow` structs
- [ ] Convergence/error handling (`PowerFlowError`)
- [ ] Golden test: `test-data/golden/powerflow/ieee-14-bus.json`
- [ ] Golden test: `test-data/golden/powerflow/ieee-30-bus.json`
- [ ] Golden test: `test-data/golden/powerflow/ieee-57-bus.json`

### `tpt-nrg-fault`
- [ ] `FaultAnalyzer` struct + `FaultType` enum (ThreePhase, LineToLine, LineToGround, DoubleLineToGround)
- [ ] Sequence network construction (positive/negative/zero)
- [ ] `calculate_fault_current()`
- [ ] `FaultResult` + `SequenceCurrents` structs
- [ ] Unit tests against known short-circuit reference cases

### Phase 2 Milestone
- [ ] Solve IEEE 14, 30, 57-bus power flow cases with <1% error vs. published results

---

## Phase 3 — Resource Modeling

### `tpt-nrg-solar`
- [ ] `SolarModel` struct (latitude, longitude, altitude_m, timezone)
- [ ] `solar_position()` — NREL Solar Position Algorithm (SPA) (via `tpt-science` astronomy)
- [ ] `SolarPosition` struct (zenith, azimuth, air_mass)
- [ ] `clear_sky_irradiance()` — GHI/DNI/DHI
- [ ] `Irradiance` struct
- [ ] `plane_of_array_irradiance()` — horizontal → tilted plane transposition
- [ ] `pv_output()` — DC output w/ temperature derating, soiling losses, inverter clipping
- [ ] Golden test: `test-data/golden/solar/nrel-spa-zenith.json`
- [ ] Golden test: `test-data/golden/solar/pv-output-derating.json`

### `tpt-nrg-wind`
- [ ] `WindModel` struct (hub_height_m, roughness_length)
- [ ] `wind_speed_at_height()` — log wind profile / power law
- [ ] `weibull_probability()` — Weibull PDF (via `tpt-math-prob-dist`)
- [ ] `WindFarm` struct + `WakeModel` enum (JensenPark, Frandsen, EddyViscosity)
- [ ] `calculate_wake_losses()` — velocity deficit per turbine
- [ ] `total_power_output()` — farm-level output after wake losses
- [ ] Golden test: `test-data/golden/wind/jensen-wake-deficit.json`
- [ ] Golden test: `test-data/golden/wind/weibull-probability.json` (verify PDF integrates to 1.0)

### `tpt-nrg-hydro`
- [ ] Hydro head/flow-rate power calculation (via `tpt-engineering` fluid dynamics)
- [ ] Integration with `GeneratorType::Hydro` in `tpt-nrg-core`
- [ ] Unit tests for hydro power output

### `tpt-nrg-load`
- [ ] `LoadModel` struct (base_load_mw, temperature_sensitivity, price_elasticity)
- [ ] `forecast_load()` — time/weather/calendar-based forecast
- [ ] `demand_response()` — price-elasticity load reduction
- [ ] Golden test data: `test-data/load-profiles/`

### Phase 3 Milestone
- [ ] Generate and validate 24-hour solar generation profile
- [ ] Generate and validate 24-hour wind generation profile

---

## Phase 4 — Storage & Microgrids

### `tpt-nrg-battery`
- [ ] `BatteryStorage` struct (capacity_mwh, power_rating_mw, state_of_charge, round_trip_efficiency, degradation_model)
- [ ] `DegradationModel` struct (cycle_life, calendar_life_years, dod_curve)
- [ ] `charge()` — SoC update w/ efficiency & limits
- [ ] `discharge()` — SoC update, min-SoC check
- [ ] `calculate_degradation()` — capacity fade from cycles/DoD/temp
- [ ] Integration hook: consume `tpt-materials::BatteryDegradationModel` capacity-fade curves
- [ ] Golden test: `test-data/golden/storage/battery-soc-cycling.json`

### `tpt-nrg-hydrogen`
- [ ] `HydrogenSystem` struct (electrolyzer, storage_tank, fuel_cell)
- [ ] `Electrolyzer` struct (efficiency kWh/kg, power_rating_mw)
- [ ] `produce_hydrogen()` — electricity → H2 mass
- [ ] `generate_electricity()` — H2 → electricity via fuel cell
- [ ] Golden test: `test-data/golden/storage/hydrogen-efficiency.json`

### `tpt-nrg-thermal-storage`
- [ ] Thermal storage charge/discharge model (via `tpt-engineering` thermodynamics)
- [ ] Unit tests for thermal storage state tracking

### `tpt-nrg-der`
- [ ] `DerAsset` struct
- [ ] `MicrogridController` struct (grid_connected, assets, control_strategy)
- [ ] `ControlStrategy` enum (GridFollowing, GridForming, DroopControl)
- [ ] Unit tests for controller state management

### `tpt-nrg-islanding`
- [ ] `detect_islanding()` — voltage/frequency/RoCoF-based loss-of-mains detection
- [ ] `transition_to_island()` — load shedding, storage/backup ramp-up, new V/f reference
- [ ] `resynchronize()` — phase/frequency/voltage matching w/ main grid
- [ ] `TransitionResult` / `SyncResult` structs
- [ ] Unit tests for islanding detection and transition logic

### `tpt-nrg-vpp`
- [ ] `VirtualPowerPlant` struct (assets, aggregation_model)
- [ ] `calculate_flexible_capacity()` — MW available for demand response
- [ ] `dispatch_assets()` — asset dispatch optimization, `DispatchPlan`
- [ ] Unit tests for aggregation and dispatch logic

### Phase 4 Milestone
- [ ] End-to-end simulation: microgrid islanding event + battery dispatch response

---

## Phase 5 — Dispatch & Economics

### `tpt-nrg-unit-commitment`
- [ ] MILP formulation (via `tpt-math-optimize-general`)
- [ ] `unit_commitment()` — on/off decisions over horizon, min up/down times, startup costs
- [ ] `UnitCommitmentResult` struct
- [ ] Golden test: `test-data/golden/dispatch/unit-commitment-24hr.json`

### `tpt-nrg-economic-dispatch`
- [ ] `economic_dispatch()` — lambda iteration / QP, minimize cost s.t. power balance
- [ ] `EconomicDispatchResult` struct (generator_outputs, total_cost, marginal_cost, losses_mw)
- [ ] `storage_arbitrage()` — optimal charge/discharge schedule from price forecast
- [ ] `ArbitragePlan` struct
- [ ] Golden test: `test-data/golden/dispatch/economic-dispatch-5gen.json`
- [ ] Unit test: marginal cost (lambda) equals incremental cost

### `tpt-nrg-reserve`
- [ ] Spinning reserve margin calculation
- [ ] Contingency reserve requirement calculation
- [ ] Unit tests for reserve margin logic

### `tpt-nrg-lcoe`
- [ ] `levelized_cost_of_energy()` — CAPEX/OPEX/fuel/generation/discount-rate LCOE formula
- [ ] `net_present_value()`
- [ ] `internal_rate_of_return()`
- [ ] Unit tests against known finance reference calculations

### `tpt-nrg-market`
- [ ] `MarketSignal` type and price-signal modeling
- [ ] Unit tests for market signal handling

### `tpt-nrg-carbon`
- [ ] `carbon_intensity()` — kg CO2/MWh from generation mix
- [ ] Unit tests for carbon intensity calculation

### Phase 5 Milestone
- [ ] Optimize 24-hour unit commitment for 10 generators, validate against golden data

---

## Phase 6 — Advanced Grid Features

### `tpt-nrg-state-estimation`
- [ ] Kalman filter-based grid state estimator (via `tpt-math-signal-filter`)
- [ ] State-of-Charge (SoC) estimation integration
- [ ] Unit tests for estimator convergence/accuracy

### `tpt-nrg-protection`
- [ ] Relay coordination model
- [ ] Protection zone / trip logic
- [ ] Unit tests for relay coordination scenarios

### Phase 6 Milestone
- [ ] Real-time state estimation demonstrated on IEEE 118-bus system

---

## Phase 7 — WASM & Edge Control

### `tpt-nrg-wasm`
- [ ] `WasmPowerFlowSolver` (`#[wasm_bindgen]`) — constructor from JSON, `solve()`
- [ ] `WasmPowerFlowResult` binding type
- [ ] `WasmMicrogridController` (`#[wasm_bindgen]`) — `step()` control-loop iteration
- [ ] `ControlAction` binding type
- [ ] Browser build pipeline (wasm-pack / trunk, whichever chosen)
- [ ] Interactive grid-planning demo (drag-and-drop buses/branches, live power flow)
- [ ] Edge microgrid control demo (bare-metal WASM target)
- [ ] Financial dashboard demo (LCOE/arbitrage in-browser, no server round-trip)

### Phase 7 Milestone
- [ ] Browser-based interactive power flow dashboard running end-to-end

---

## Phase 8 — Ecosystem Integration

- [ ] Integrate `tpt-materials::BatteryDegradationModel` capacity-fade curves into `tpt-nrg-battery`
- [ ] Integrate `tpt-transport` wind turbine power curves + wake models into `tpt-nrg-wind`
- [ ] Integrate `tpt-electronics` inverter clipping limits + PV thermal derating into `tpt-nrg-solar`
- [ ] Cross-repo integration tests (materials/transport/electronics ↔ energy)
- [ ] End-to-end "Energy Cycle" example: material degradation → device physics → grid dispatch

### Phase 8 Milestone
- [ ] Full end-to-end Energy Cycle simulation passes

---

## Phase 9 — Documentation, Release & v1.0

- [ ] `docs/book` (mdBook or equivalent) — architecture, crate guides, tutorials
- [ ] `docs/api` — generated API docs (`cargo doc`) published via `docs.yml` workflow
- [ ] Finalize `README.md` crate status table (Stable/Alpha/Planned) per Section 13
- [ ] RFC 0004 — hydrogen electrolysis (if not resolved during Phase 4)
- [ ] RFC 0005 — virtual power plant (if not resolved during Phase 4)
- [ ] IEC 61850 / IEC 61970 (CIM) standards compliance review
- [ ] NERC standards compliance review
- [ ] Full `cargo deny check licenses` audit across all crates
- [ ] Publish all crates to crates.io
- [ ] Tag `v1.0.0`
- [ ] Establish ongoing SemVer / 6-week release cadence
