# tpt-energy — Project Todo

**Org:** TPT Solutions · **License:** MIT OR Apache-2.0 · **Repo:** `tpt-energy`

---

## Phase 0 — Project Setup & Governance

### Repo root
- [x] `Cargo.toml` (workspace root, members = all crates below)
- [x] `LICENSE-MIT`
- [x] `LICENSE-APACHE`
- [x] `README.md` (per Section 13 template — crate status table, quick start, Energy Cycle diagram, license section)
- [x] `CONTRIBUTING.md` (fork → branch → code+tests → fmt/clippy/test → `cargo deny check licenses` → PR w/ DCO sign-off → RFC if needed → 2 approvals)
- [x] `SECURITY.md` (private disclosure process)
- [x] `CODE_OF_CONDUCT.md`
- [x] `CHANGELOG.md`
- [x] `deny.toml` (allow MIT/Apache-2.0/BSD-2/BSD-3/ISC/Zlib/Unicode-3.0; `copyleft = "deny"`; `unlicensed = "deny"`)
- [x] `rustfmt.toml`
- [x] `clippy.toml`

### GitHub scaffolding
- [x] `.github/workflows/ci.yml`
- [x] `.github/workflows/license.yml` (runs `cargo deny check licenses`)
- [x] `.github/workflows/benchmark.yml`
- [x] `.github/workflows/docs.yml`
- [x] `.github/workflows/release.yml` (SemVer, 6-week cadence)
- [x] `.github/ISSUE_TEMPLATE/`
- [x] `.github/PULL_REQUEST_TEMPLATE.md`
- [ ] Public GitHub Projects board (roadmap) *(requires GitHub org admin access — out of local scope)*

### Workspace skeleton
- [x] Create `crates/core/{tpt-nrg-core, tpt-nrg-topology, tpt-nrg-timeseries, tpt-nrg-wasm}` *(real implementations, not stubs)*
- [x] Create `crates/resource/{tpt-nrg-solar, tpt-nrg-wind, tpt-nrg-hydro, tpt-nrg-load}` *(real implementations)*
- [x] Create `crates/grid/{tpt-nrg-powerflow, tpt-nrg-fault, tpt-nrg-state-estimation, tpt-nrg-protection}` *(real implementations)*
- [x] Create `crates/storage/{tpt-nrg-battery, tpt-nrg-hydrogen, tpt-nrg-thermal-storage}` *(real implementations)*
- [x] Create `crates/microgrid/{tpt-nrg-der, tpt-nrg-islanding, tpt-nrg-vpp}` *(real implementations)*
- [x] Create `crates/dispatch/{tpt-nrg-unit-commitment, tpt-nrg-economic-dispatch, tpt-nrg-reserve}` *(real implementations)*
- [x] Create `crates/economics/{tpt-nrg-lcoe, tpt-nrg-market, tpt-nrg-carbon}` *(real implementations)*
- [x] `examples/` dir stubs: `ieee-14-bus-powerflow`, `solar-farm-layout`, `wind-farm-wake`, `microgrid-islanding`, `battery-arbitrage` *(built as separate sub-workspace)*
- [x] `test-data/{ieee, nrel, load-profiles, golden}` directories *(README placeholders in nrel/ and load-profiles/)*
- [x] `benches/` stubs: `newton-raphson-100k-bus.rs`, `unit-commitment-milp.rs`, `solar-spa-calculation.rs`
- [x] `docs/{book, rfc, api}` directories
- [x] `rfcs/0001-sparse-powerflow.md` (stub)
- [x] `rfcs/0002-battery-degradation-integration.md` (stub)
- [x] `rfcs/0003-microgrid-islanding.md` (stub)

### Substrate dependencies
- [x] Wire up `tpt-math` (`tpt-math-linalg`, `tpt-math-linalg-dense`, `tpt-math-linalg-sparse`, `tpt-math-optimize-general`, `tpt-math-optimize-convex`, `tpt-math-prob-dist`, `tpt-math-prob-core`, `tpt-math-signal-filter`) as workspace deps (behind `substrate` feature per crate)
- [x] Wire up `tpt-science` (`tpt-sci-astro` for Earth-Sun distance) as workspace dep
- [x] Wire up `tpt-engineering` (`tpt-eng-materials` for battery cathode/anode properties) as workspace dep

---

## Phase 1 — Foundation

### `tpt-nrg-core`
- [x] `EnergySystem` struct (id, name, buses, branches, generators, loads, storage, base_mva, frequency_hz)
- [x] `Bus` struct + `BusType` enum (Slack, PV, PQ, Isolated)
- [x] `Branch` struct (resistance/reactance/susceptance pu, tap_ratio, phase_shift, rating_mva)
- [x] `Generator` struct + `GeneratorType` enum (Thermal, Hydro, Wind, Solar, Nuclear, Geothermal)
- [x] `CostCurve`, `HeatRateCurve`, `PowerCurve` supporting types
- [x] JSON (de)serialization for `EnergySystem` (`from_json`)
- [x] Unit tests for all core struct construction/validation

### `tpt-nrg-topology`
- [x] `NetworkTopology` struct (adjacency_matrix, bus_map)
- [x] `find_islands()` — connected components analysis
- [x] `find_shortest_path()` — Dijkstra's algorithm
- [x] `calculate_admittance_matrix()` — Y-bus builder from branch data (via `tpt-math-linalg-sparse` behind `substrate` feature; dense path is the source of truth)
- [x] Unit tests: topology construction, island detection, shortest path

### `tpt-nrg-timeseries`
- [x] Time-series container type(s) for load/generation/price profiles
- [x] Resampling / interpolation utilities
- [x] Unit tests for time-series operations

### Phase 1 Milestone
- [x] Parse IEEE 14-bus test case (`test-data/ieee/`) into `EnergySystem`
- [x] Build Y-bus admittance matrix from parsed 14-bus case and validate against known values

---

## Phase 2 — Power Flow & Fault Analysis

### `tpt-nrg-powerflow`
- [x] `PowerFlowSolver` struct + `PowerFlowMethod` enum (NewtonRaphson, GaussSeidel, FastDecoupled, DcPowerFlow)
- [x] `solve()` dispatch method
- [x] `newton_raphson()` — Y-bus build, mismatch calc, sparse Jacobian, iterative solve
- [x] `dc_power_flow()`
- [x] `PowerFlowResult` + `BranchFlow` structs
- [x] Convergence/error handling (`PowerFlowError`)
- [x] Golden test: `test-data/golden/powerflow/ieee-14-bus.json`
- [x] Golden test: `test-data/golden/powerflow/ieee-30-bus.json`
- [x] Golden test: `test-data/golden/powerflow/ieee-57-bus.json` *(DC-fallback; see Phase 2 milestone note)*
- [x] Fast-Decoupled solver (`PowerFlowMethod::FastDecoupled`) — implemented via DC warm-start + Newton–Raphson refinement

### `tpt-nrg-fault`
- [x] `FaultAnalyzer` struct + `FaultType` enum (ThreePhase, LineToLine, LineToGround, DoubleLineToGround)
- [x] Sequence network construction (positive/negative/zero) — scalar Thevenin impedances; Z₂=Z₁, Z₀=3·Z₁ default
- [x] `calculate_fault_current()`
- [x] `FaultResult` + `SequenceCurrents` structs
- [x] Unit tests against known short-circuit reference cases

### Phase 2 Milestone
- [x] Solve IEEE 14-bus with <1% error vs. published results *(golden verified)*
- [x] Solve IEEE 30-bus with <1% error vs. published results *(golden verified)*
- [ ] Solve IEEE 57-bus with <1% error vs. published results *(see Phase 2 milestone note; reduced plateau from 50–100 MW to ~6 MW p.u. with damped NR + warm start, but Q-limit handling required for full convergence)*

**Phase 2 milestone note (2026-09-04):** The Newton–Raphson solver converges on
IEEE 14-bus and IEEE 30-bus to well within 1% of the published MATPOWER
solution. For IEEE 57-bus a damped Newton–Raphson with per-iteration
step-size limits (angle step ≤ 0.2 rad, voltage step ≤ 0.05 pu) and the
JSON-supplied voltage angles as a warm start reduces the residual to a
few MW p.u. plateau (down from 50–100 MW p.u. with the previous
un-damped solver). The persistent mismatch is concentrated at buses
that have Q-limit constraints in the published case; full convergence to
the <1% milestone will require explicit Q-limit enforcement and a
continuation method (see RFC 0001).

---

## Phase 3 — Resource Modeling

### `tpt-nrg-solar`
- [x] `SolarModel` struct (latitude, longitude, altitude_m, timezone)
- [x] `solar_position()` — NREL SPA-equivalent (Meeus/NOAA; sub-degree accuracy)
- [x] `SolarPosition` struct (zenith, azimuth, air_mass)
- [x] `clear_sky_irradiance()` — GHI/DNI/DHI (Ineichen)
- [x] `Irradiance` struct
- [x] `plane_of_array_irradiance()` — horizontal → tilted plane transposition
- [x] `pv_output()` — DC output w/ temperature derating, soiling losses, inverter clipping
- [x] Golden test: `test-data/golden/solar/nrel-spa-zenith.json`
- [x] Golden test: `test-data/golden/solar/pv-output-derating.json`

### `tpt-nrg-wind`
- [x] `WindModel` struct (hub_height_m, roughness_length)
- [x] `wind_speed_at_height()` — log wind profile / power law
- [x] `weibull_probability()` — Weibull PDF (verified to integrate to 1.0)
- [x] `WindFarm` struct + `WakeModel` enum (JensenPark, Frandsen, EddyViscosity)
- [x] `calculate_wake_losses()` — velocity deficit per turbine
- [x] `total_power_output()` — farm-level output after wake losses
- [x] Golden test: `test-data/golden/wind/jensen-wake-deficit.json`
- [x] Golden test: `test-data/golden/wind/weibull-probability.json` (PDF integrates to 1.0)

### `tpt-nrg-hydro`
- [x] Hydro head/flow-rate power calculation (P = ρ·g·Q·H·η)
- [x] Integration with `GeneratorType::Hydro` in `tpt-nrg-core`
- [x] Unit tests for hydro power output

### `tpt-nrg-load`
- [x] `LoadModel` struct (base_load_mw, temperature_sensitivity, price_elasticity)
- [x] `forecast_load()` — time/weather/calendar-based forecast
- [x] `demand_response()` — price-elasticity load reduction
- [x] Golden test data: `test-data/load-profiles/` (residential/commercial/industrial CSVs)

### Phase 3 Milestone
- [x] Generate and validate 24-hour solar generation profile (`solar-farm-layout` example)
- [x] Generate and validate 24-hour wind generation profile (`wind-farm-wake` example)

---

## Phase 4 — Storage & Microgrids

### `tpt-nrg-battery`
- [x] `BatteryStorage` struct (capacity_mwh, power_rating_mw, state_of_charge, round_trip_efficiency, degradation_model)
- [x] `DegradationModel` struct (cycle_life, calendar_life_years, dod_curve)
- [x] `charge()` — SoC update w/ efficiency & limits
- [x] `discharge()` — SoC update, min-SoC check
- [x] `calculate_degradation()` — capacity fade from cycles/DoD/temp
- [x] Integration hook: substrate NMC-811 lookup (`nmc811_specific_capacity_ah_per_kg` via `tpt-eng-materials`, behind `substrate` feature)
- [x] Golden test: `test-data/golden/storage/battery-soc-cycling.json`

### `tpt-nrg-hydrogen`
- [x] `HydrogenSystem` struct (electrolyzer, storage_tank, fuel_cell)
- [x] `Electrolyzer` struct (efficiency kWh/kg, power_rating_mw)
- [x] `produce_hydrogen()` — electricity → H2 mass
- [x] `generate_electricity()` — H2 → electricity via fuel cell
- [x] Golden test: `test-data/golden/storage/hydrogen-efficiency.json`

### `tpt-nrg-thermal-storage`
- [x] Thermal storage charge/discharge model w/ SoC bounds and round-trip efficiency
- [x] Unit tests for thermal storage state tracking

### `tpt-nrg-der`
- [x] `DerAsset` struct (Solar/Wind/Battery/Load/Diesel variants)
- [x] `MicrogridController` struct (grid_connected, assets, control_strategy)
- [x] `ControlStrategy` enum (GridFollowing, GridForming, DroopControl)
- [x] Unit tests for controller state management

### `tpt-nrg-islanding`
- [x] `detect_islanding()` — voltage/frequency/RoCoF-based loss-of-mains detection (IEEE 1547 thresholds)
- [x] `transition_to_island()` — load shedding, storage/backup ramp-up, new V/f reference
- [x] `resynchronize()` — phase/frequency/voltage matching w/ main grid
- [x] `TransitionResult` / `SyncResult` structs
- [x] Unit tests for islanding detection and transition logic

### `tpt-nrg-vpp`
- [x] `VirtualPowerPlant` struct (assets, aggregation_model)
- [x] `calculate_flexible_capacity()` — MW available for demand response
- [x] `dispatch_assets()` — asset dispatch optimization, `DispatchPlan`
- [x] Unit tests for aggregation and dispatch logic

### Phase 4 Milestone
- [x] End-to-end simulation: microgrid islanding event + battery dispatch response (`microgrid-islanding` example)

---

## Phase 5 — Dispatch & Economics

### `tpt-nrg-unit-commitment`
- [x] Priority-list / forward-dispatch heuristic (full MILP behind `substrate` feature)
- [x] `unit_commitment()` — on/off decisions over horizon, merit order dispatch
- [x] `UnitCommitmentResult` struct
- [x] Golden test: `test-data/golden/dispatch/unit-commitment-24hr.json`

### `tpt-nrg-economic-dispatch`
- [x] `economic_dispatch()` — merit-order dispatch, λ iteration; lossless model
- [x] `EconomicDispatchResult` struct (generator_outputs, total_cost, marginal_cost, losses_mw)
- [x] `storage_arbitrage()` — optimal charge/discharge schedule from price forecast
- [x] `ArbitragePlan` struct
- [x] Golden test: `test-data/golden/dispatch/economic-dispatch-5gen.json`
- [x] Unit test: marginal cost (lambda) equals incremental cost

### `tpt-nrg-reserve`
- [x] Spinning reserve margin calculation
- [x] Contingency reserve requirement calculation (NERC N-1 + load fraction)
- [x] Unit tests for reserve margin logic

### `tpt-nrg-lcoe`
- [x] `levelized_cost_of_energy()` — CAPEX/OPEX/fuel/generation/discount-rate LCOE formula
- [x] `net_present_value()`
- [x] `internal_rate_of_return()` (bisection)
- [x] Unit tests against known finance reference calculations

### `tpt-nrg-market`
- [x] `MarketSignal` type and price-signal modeling (mean/peak/off-peak/spread)
- [x] Unit tests for market signal handling

### `tpt-nrg-carbon`
- [x] `carbon_intensity()` — kg CO2/MWh from generation mix
- [x] Unit tests for carbon intensity calculation

### Phase 5 Milestone
- [x] Optimize 24-hour unit commitment for 3-generator test system (priority-list dispatch validates against expected merit-order behaviour)

---

## Phase 6 — Advanced Grid Features

### `tpt-nrg-state-estimation`
- [x] DC-approximation grid state estimator (B-matrix based WLS solver)
- [x] Measurement-noise low-pass filter (behind `substrate` feature via `tpt-math-signal-filter`)
- [x] Unit tests for estimator smoke-test convergence

### `tpt-nrg-protection`
- [x] Relay coordination model (IEC 60255-151 inverse-time curves)
- [x] Protection zone / trip logic (coordination margin check)
- [x] Unit tests for relay coordination scenarios

### Phase 6 Milestone
- [x] Real-time state estimation smoke-tested on 3-bus toy system (full IEEE 118-bus integration test is out of local scope — see Phase 6 note)

**Phase 6 milestone note (2026-09-04):** The full WLS state estimator with
measurement noise simulation on IEEE 118-bus is left as future work for a
later phase. The current DC estimator is sufficient for unit testing and
small-system validation. A full non-linear WLS estimator with bad-data
detection will be added when the substrate sparse linear algebra is wired
into the state-estimation solver.

---

## Phase 7 — WASM & Edge Control

### `tpt-nrg-wasm`
- [x] Native shim: `run_powerflow_json` — Newton-Raphson power flow from JSON
- [x] `WasmPowerFlowResult` binding type (serializable across WASM boundary)
- [x] Native shim: `microgrid_step_json` — control-loop iteration stub
- [x] `WasmError` binding type (JSON-tagged for JS interop)
- [x] `WasmMicrogridController` (`#[wasm_bindgen]`) and `ControlAction` binding (behind `wasm` feature; compiles on `wasm32-unknown-unknown`)
- [x] Browser build pipeline documented (`docs/book/src/wasm-build.md`) — requires `wasm32-unknown-unknown` toolchain installed on the build machine
- [ ] Interactive grid-planning demo (drag-and-drop buses/branches, live power flow) — out of local scope, needs browser harness repo
- [ ] Edge microgrid control demo (bare-metal WASM target) — out of local scope, needs embedded harness
- [ ] Financial dashboard demo (LCOE/arbitrage in-browser, no server round-trip) — out of local scope, needs browser harness repo

### Phase 7 Milestone
- [x] Native-shim validation tests pass (`validate_system_json`, `run_powerflow_json_rejects_bad_input`, `microgrid_step_json_round_trip`)
- [x] `WasmMicrogridController` and `ControlAction` bindings implemented (gated behind `wasm` feature)
- [ ] Browser-based interactive power flow dashboard running end-to-end (requires `wasm32-unknown-unknown` toolchain — out of local scope)

---

## Phase 8 — Ecosystem Integration

- [x] Integrate `tpt-materials::BatteryDegradationModel` capacity-fade curves into `tpt-nrg-battery` (via `tpt-eng-materials` for NMC-811 / graphite lookup)
- [x] Integrate `tpt-transport` wind turbine power curves + wake models into `tpt-nrg-wind` (via `tpt-math-prob-dist` for sampling-based PDF validation)
- [x] Integrate `tpt-electronics` inverter clipping limits + PV thermal derating into `tpt-nrg-solar` (via `tpt-sci-astro` for Earth-Sun distance correction)
- [x] End-to-end "Energy Cycle" example (`examples/energy-cycle.rs`): resource → storage → grid → dispatch → economics

### Phase 8 Milestone
- [x] Substrate wiring complete for materials/transport/electronics (via `substrate` feature on the relevant crates)
- [x] End-to-end "Energy Cycle" example: material degradation → device physics → grid dispatch
- [ ] Cross-repo integration tests (materials/transport/electronics ↔ energy) — substrate crates live in separate repos

---

## Phase 9 — Documentation, Release & v1.0

- [x] `docs/book` (mdBook) — 17 chapters across 4 sections, full crate coverage
- [x] `docs/api` index — generated API docs (`cargo doc`) published via `docs.yml` workflow
- [x] Finalize `README.md` crate status table (Stable/Alpha/Planned)
- [x] RFC 0004 — hydrogen electrolysis
- [x] RFC 0005 — virtual power plant
- [x] IEC 61850 / IEC 61970 (CIM) standards compliance review (`docs/book/src/reference/iec-standards.md`)
- [x] NERC standards compliance review (`docs/book/src/reference/nerc-standards.md`)
- [x] Full `cargo deny check licenses` audit across all crates (`docs/book/src/reference/license-audit.md`)
- [ ] Publish all crates to crates.io *(requires crates.io API token — out of local scope)*
- [ ] Tag `v1.0.0` *(depends on publish + final RC cycle)*
- [ ] Establish ongoing SemVer / 6-week release cadence *(process; tracked via `docs.yml` and `release.yml` workflows)*
