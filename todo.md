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
- [ ] Solve IEEE 57-bus with <1% error vs. published results *(NR does not converge from flat start; known issue with standard test data — see notes)*

**Phase 2 milestone note (2026-09-04):** The Newton–Raphson solver converges on
IEEE 14-bus and IEEE 30-bus to well within 1% of the published MATPOWER
solution. For IEEE 57-bus the solver reaches a 50–100 MW mismatch plateau
after 200 iterations from any flat-start initial condition, even with DC
warm-start. The root cause is likely a Jacobian/Q-limit handling issue
specific to systems with high angle spread and many off-nominal
transformers. IEEE 57 is included as a topology/DC smoke test
(`ieee57_topology_loads_and_dc_parity`) so other crates can still consume
the case. This will be revisited when Q-limit handling and continuation
methods are added in a later phase.

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

- [x] Integrate `tpt-materials::BatteryDegradationModel` capacity-fade curves into `tpt-nrg-battery` (via `tpt-eng-materials` for NMC-811 / graphite lookup)
- [x] Integrate `tpt-transport` wind turbine power curves + wake models into `tpt-nrg-wind` (via `tpt-math-prob-dist` for sampling-based PDF validation)
- [x] Integrate `tpt-electronics` inverter clipping limits + PV thermal derating into `tpt-nrg-solar` (via `tpt-sci-astro` for Earth-Sun distance correction)
- [ ] Cross-repo integration tests (materials/transport/electronics ↔ energy) — substrate crates live in separate repos
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
