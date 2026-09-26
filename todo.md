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
- [x] Solve IEEE 57-bus with <1% error vs. published results *(closed 2026-09-18: PV↔PQ Q-limit enforcement + corrected shunt units; AC NR converges in 5 iterations and reproduces the published MATPOWER case57 solution — losses ≈ 27.86 MW, slack ≈ 478.66 MW / 128.85 MVAr — within 1%; asserted in `ieee57_ac_matches_golden_and_published`)*

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

**Resolution (2026-09-18):** Q-limit enforcement (PV ↔ PQ switching with a
near-converged-iterate outer loop, Dommel–Tinney style) plus a fix to the
test-case shunt susceptances (MATPOWER MVAr values had been stored in
per-unit fields, 100× too large) close the milestone without a
continuation method. IEEE 57-bus converges in 5 iterations to ~1e-11
mismatch and matches the published solution within 1% on the asserted
anchors.

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
- [x] Browser-based interactive power flow dashboard running end-to-end *(built in the second pass: `playground/` loads the `wasm32-unknown-unknown` build, solves a chosen or pasted case, and draws the voltage profile and single-line diagram, with `playground/tests/smoke.mjs` covering the same module from Node and a GitHub Pages workflow that deploys it. What remains out of scope is the richer drag-and-drop planner listed above.)*

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

---

## Phase 10 — CI Health & Quality Backlog

**Audit note (2026-09-16):** All four jobs in `.github/workflows/ci.yml` plus the
`docs.yml` workflows currently fail on `master`. Every item below cites the
command that reproduces the failure locally. The fix pass that landed as
`92f452f` ("Fix correctness bugs across power flow, dispatch, and resource
crates") did not address any of these.

**Resolution note (2026-09-18):** every item in this phase is now closed.
All gates pass locally: `cargo fmt --all -- --check`, `cargo clippy
--workspace --all-targets --all-features -- -D warnings`,
`RUSTFLAGS="-D warnings" cargo test --workspace --all-features` (168 tests),
`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`,
and the examples sub-workspace builds and runs.

### `fmt` job (`.github/workflows/ci.yml`)
- [x] Add `.gitattributes` (`* text=auto eol=lf`) and renormalize the tree *(landed in `4b79581`)*
- [x] Resolve the nightly-only `rustfmt.toml` options *(dropped; stable-only config)*
- [x] Run `cargo fmt --all` and commit the formatting-only diff *(clean as of the 2026-09-18 pass)*

### `clippy` job
- [x] Reconcile `-D warnings` with the per-crate lint policy: `cargo clippy --workspace --all-targets --all-features -- -D warnings` emits 107 errors, because all 24 crates enable `[lints.clippy] pedantic = { level = "warn", priority = -1 }` plus `missing_errors_doc` / `missing_panics_doc` *(decision: keep the strict gate — the pedantic set was fixed, not narrowed)*
- [x] Decide the policy for that gate: fix the pedantic set, or narrow CI to the default lint groups and keep pedantic advisory *(fixed)*
- [x] Clear the high-count pedantic groups (must_use / doc_markdown / missing_errors_doc / float casts / …) — all resolved, with a small number of documented, intentional `#[allow]`s (unit-suffixed field names, bounded casts)
- [x] Replace the lossy `as f64` / `as usize` / `as u64` casts with checked or documented conversions *(crate-local `count_to_f64`-style helpers or `try_from`)*

### `test` job
- [x] Fix the 17 rustc warnings that `RUSTFLAGS: -D warnings` promotes to hard errors *(landed in `92f452f`; verified green)*
- [x] Triage the unused parameters above: wire up or explicitly `_`-prefix with a comment *(thrust coefficient is now threaded through the wake models)*

### `docs` job
- [x] Move `docs/book/SUMMARY.md` to `docs/book/src/SUMMARY.md` *(moved; chapter links fixed to resolve inside `src/`)*
- [x] Fix the rustdoc `-D warnings` failures (unresolved intra-doc links, unnecessary parens, unused imports) *(clean)*

### Documentation / CHANGELOG drift (introduced by `92f452f`)
- [x] Correct `docs/book/src/crate-dispatch.md`: reserve section now documents `spinning_reserve_margin(online, output)` (headroom-only) and `assess_reserves`/`ReserveAssessment`
- [x] Correct the wake-model claims in `docs/book/src/crate-wind.md`, `crate-status.md`, `CHANGELOG.md` and `README.md` *(Frandsen and EddyViscosity are now real, golden-tested implementations — the docs were corrected to describe them accurately)*
- [x] Document the API changes from `92f452f` wherever the book references them: `off_peak_price()` returns `Option<f64>` (documented in `crate-economics.md`), reserve API is `ReserveAssessment` / `assess_reserves`
- [x] Add `### Fixed` (and `### Changed`) sections to the root `CHANGELOG.md` covering the correctness pass and this quality pass
- [x] Re-verify the `Stable` status claims in `README.md` and `docs/book/src/crate-status.md` against the current public API surface

### Code gaps and dead code
- [x] `tpt-nrg-wind`: thread the turbine's thrust coefficient through the Jensen deficit *(landed in `92f452f`; covered by golden tests)*
- [x] Implement `WakeModel::Frandsen` (rotor-equivalent source, two-zone deficit, partial-rotor overlap) and `WakeModel::EddyViscosity` (explicit cylindrical diffusion march), with golden fixtures for each *(fixtures are generated by `generate-wind-goldens` from the library itself and rounded to 9 decimals for cross-platform stability)*
- [x] Remove dead code: `turbine.rs` `_trapezoid` / `_gauss_helper`, `wake.rs` `dir_rad_for_test`, `graph.rs` `_ensure` *(verified removed)*
- [x] `test-data/nrel/`: the placeholder README claimed files that did not exist; it now states clearly that no NREL datasets are committed and points to `test-data/golden/solar/`

### CI coverage gaps
- [x] Build the `examples/` sub-workspace in CI: new `examples` job runs `cargo check --manifest-path examples/Cargo.toml --all-targets` and executes `ieee-14-bus-powerflow` *(this immediately caught an `energy-cycle` regression — it now attaches cost curves before dispatching, matching the post-`92f452f` must-run semantics)*
- [x] Wire the `benches/` stubs into the workspace with `criterion` and `[[bench]]` targets: `newton_raphson` (30/100/200-bus meshed lattices) in `tpt-nrg-powerflow`, `solar_spa` (8760-hour position + PV output) in `tpt-nrg-solar`, `unit_commitment` (24 h, 10/30/100 units) in `tpt-nrg-unit-commitment`; `benchmark.yml` moved to the stable toolchain
- [x] Add a CI golden-drift guard: a `golden-drift` job re-runs all three golden generators and fails on `git diff test-data`; generators round emitted values to 9 decimals so libm ULP differences across platforms cannot produce false positives (idempotency verified locally)

**Still open from earlier phases:** the full non-linear WLS state estimator with
IEEE 118-bus validation (Phase 6 milestone note), the Phase 7 browser demos
(out of local scope), cross-repo substrate integration tests (out of local
scope), crates.io publishing + `v1.0.0` tag (requires API token), and the
GitHub Projects roadmap board (requires org admin access).

---

## Phase 11 — Adoption, Interop & Quality Backlog

**Origin note (2026-09-26):** items below come from a platform review focused
on bugs/gaps and, in particular, what would make adoption faster and easier
(examples, templates, automation). The codebase itself has no TODOs,
`unimplemented!()`, or ignored tests — these are surface-area gaps and
polish items, not fixes for broken code.

### Correctness / cleanup
- [x] `tpt-nrg-topology/src/graph.rs:120` — document (or `.expect(...)`-annotate) the `prev[cur].unwrap()` invariant in BFS path reconstruction; it's safe today (guarded by the `visited[goal]` check) but undocumented
- [x] `tpt-nrg-wasm`: replace stringified `JsValue::from(e.to_string())` errors with the existing `WasmError` (`{kind, message}`) type across the `#[wasm_bindgen]` boundary so JS callers can discriminate error kinds
- [x] RFC: unify error handling — decide whether `tpt-nrg-powerflow::PowerFlowError`, `tpt-nrg-economic-dispatch::DispatchError`, and the other per-crate `thiserror` enums (timeseries, topology, wasm, battery) should wrap/extend `tpt-nrg-core::CoreError` instead of each rolling an independent taxonomy *(decided and written up in `rfcs/0006-unify-error-handling.md`; adoption is staged and seeded as good first issues. `WasmError`, `InteropError`, and the Python `error_kind` already implement the proposed `kind` + `message` shape, so the first three layers are in place)*

### CLI
- [x] New `tpt-nrg` CLI binary/crate: `tpt-nrg run --system case.json --method newton-raphson --format table|json` and `tpt-nrg convert --from matpower --to json`
- [x] Wire prebuilt binary releases for the CLI into the existing `release.yml` (e.g. via `cargo-dist` or `cargo binstall` support) *(a `cli-binaries` job builds Linux x86-64/aarch64, macOS aarch64, and Windows x86-64 with `--locked` and attaches them to the release)*

### Interoperability
- [x] Promote `tools/matpower_to_json.py` into a first-class Rust crate (`tpt-nrg-interop`) with two-way MATPOWER ⇄ JSON conversion (not a standalone Python script)
- [x] Add PSS/E import/export to `tpt-nrg-interop`
- [x] Add CIM (IEC 61970) import/export to `tpt-nrg-interop`, building on the existing `docs/book/src/reference/iec-standards.md` review
- [x] Add YAML and CSV import for `EnergySystem` construction (JSON-only today)

### Language bindings & distribution
- [x] Python bindings via `pyo3`/`maturin`, published to PyPI *(the crate, the `pyproject.toml`, and a 33-check API smoke test in CI are in place; `release.yml` now has a `publish-pypi` job that runs `maturin publish` when the `PYPI_API_TOKEN` secret is set — the secret is `PYPI_API_TOKEN`, not `TWINE`, and the upload happens on a `v*.*.*` tag)*
- [x] Package the `tpt-nrg-wasm` `wasm-pack` output as a publishable npm package; check in or CI-generate the `.d.ts` TypeScript type definitions *(`crates/core/tpt-nrg-wasm/npm/` holds the published `package.json` and README plus a checked-in `tpt_nrg_wasm.d.ts`; `tools/build-npm-package.sh` regenerates the package and `--check` fails CI when the checked-in declarations are stale. The crate needed `crate-type = ["cdylib", "rlib"]` before `wasm-pack` would build it at all.)*
- [x] Expand the WASM surface beyond power flow/validation/microgrid-step-stub to cover dispatch, unit commitment, LCOE, and carbon intensity
- [x] Implement real control logic for `microgrid_step_json` (currently an explicit placeholder returning empty actions)

### Visualization
- [x] New minimal visualization crate: SVG single-line diagram + voltage/loading heatmap rendered from a `PowerFlowResult`

### Playground / demo
- [x] Hosted interactive browser playground (mdBook + WASM + the new visualization crate): pick an IEEE test case or paste a MATPOWER case, see power flow and voltage profile in-browser *(closes the browser-harness gap already flagged in Phase 7)* — the pieces it needs now exist (`wasm_visualize_json`, `wasm_economic_dispatch_json`, and the typed `WasmError` the UI switches on), so this is a static-site-plus-CI job rather than new analysis code *(`playground/`, deployed to GitHub Pages by `.github/workflows/playground.yml`; see the Phase 12 entry for the details)*

### Adoption / usability
- [x] `cargo generate` project template ("new energy system project"): scaffolded `Cargo.toml` pinned to workspace crate versions, example `system.json`, and a `main.rs` that loads + solves it *(`templates/energy-system/`, also rendered by `tpt-nrg new`; see the Phase 12 entry)*
- [x] Task-shaped worked examples: "import a MATPOWER case and run a contingency analysis," "size a battery for peak shaving from a load CSV" *(`examples/contingency-analysis.rs` verifies the MATPOWER import against the committed JSON case, then runs an N-1 sweep that ranks overloads and reports a short-circuit level; `examples/battery-sizing.rs` sizes by simulating the duty cycle and then confirms the size against the real `BatteryStorage` SoC model. Both run in the CI `examples` job. The battery example deliberately reports a bad investment: at a 0.82 load factor, holding a flat ceiling needs a 106 MWh pack, so the payback exceeds the asset life. That is the correct answer for this profile, and the example says so instead of picking a flattering size.)*
- [x] Add README badges (build status, crates.io version, Codecov, docs.rs) *(added in Phase 10, and corrected in Phase 12: nothing is published to any registry yet, so the registry version badges were removed rather than left pointing at 404s. `tools/check-badges.py` now fails CI if one comes back before it is true)*

### Automation
- [x] Automate cross-crate changelog/version bumps (e.g. `release-plz` or `cargo-smart-release`), given the workspace already uses shared `[workspace.package]` versioning *(`release-plz.toml` plus `.github/workflows/release-plz.yml`; publishing stays tag-driven, see the Phase 12 entry)*
- [x] Seed a small batch of labeled "good first issue"s (e.g. the error-unification RFC and WASM error-typing fix above) to give new contributors an entry point *(five items in `.github/good-first-issues.yml`)*

**Phase 11 status (2026-09-26):** all 18 items are now closed — the npm
package, the browser playground, the `cargo generate` template, and the release
automation were finished in the second pass (see Phase 12). Every gate is green
locally: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`RUSTFLAGS="-D warnings" cargo test --workspace --all-features` (247 unit and
integration tests, plus 3 doc tests), `RUSTDOCFLAGS="-D warnings" cargo doc`,
the `wasm32-unknown-unknown` build, the Python wheel plus its 33-check smoke
test, and the `examples/` sub-workspace (including both worked examples, which
CI now runs). Publishing to crates.io, PyPI, and npm still needs registry
secrets and is tracked in Phase 9; the jobs that do it are now in place.

---

## Phase 12 — Release Reliability, Adoption & Innovation Backlog

**Status (2026-09-26, second pass):** 10 of the 14 items below are closed, plus
the four carried over from Phase 11 (npm package, browser playground,
`cargo generate` template, release automation). Every gate is green locally:
`cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`RUSTFLAGS="-D warnings" cargo test --workspace --all-features` (61 test
targets, no failures), `RUSTDOCFLAGS="-D warnings" cargo doc`, the
`wasm32-unknown-unknown` build, the `examples/` sub-workspace,
`tools/publish-dry-run.sh`, the playground smoke test
(`node playground/tests/smoke.mjs`), and `tpt-nrg new --local` followed by
`cargo run` on the generated project. The remaining items need registry
credentials or a browser and say so where they are listed.

**Origin note (2026-09-26):** items below come from a second platform review
focused on verifying Phase 11's claims against the actual workflow/README
code, plus new bug findings, innovation ideas, and adoption suggestions
outside the existing backlog.

### Bugs (verified against current code)
- [x] `release.yml` publish job is broken: for every crate in the matrix it
  runs 8 `cargo publish` steps, one per category `working-directory`
  (`crates/core`, `crates/resource`, `crates/grid`, `crates/storage`,
  `crates/microgrid`, `crates/dispatch`, `crates/economics`,
  `crates/interop`), but each crate only lives in one of those directories.
  GitHub Actions fails a step outright when `working-directory` doesn't
  exist, so the job will fail for every crate on the next `v*.*.*` tag push,
  which also blocks `github-release` (`needs: [publish, cli-binaries]`).
  Fix by discovering each crate's real manifest path (e.g. via `cargo
  metadata --workspace --format-version1` or a `find`-based lookup) instead
  of the hard-coded 8-directory matrix. *(fixed: the matrix is gone. The job
  lists the crates with `tools/workspace-crates.py` — which reads
  `cargo metadata`, so a crate cannot be silently skipped — and then runs a
  single `cargo publish --workspace`, which publishes in dependency order. A
  missing token now fails with an explicit message instead of an
  unauthenticated upload.)*
- [x] `todo.md` (Phase 11) claims PyPI publishing "runs from the release
  workflow" once a `TWINE` secret is set, but no `maturin publish` /
  `twine upload` / `wasm-pack publish` step exists anywhere in
  `.github/workflows/*.yml` — `maturin` only appears in `ci.yml` as a
  build-and-smoke-test step. Add the real publish jobs (gated on
  `secrets.PYPI_API_TOKEN` / `secrets.NPM_TOKEN`) or correct the claim.
  *(fixed: `release.yml` gained `publish-pypi` (`maturin publish`) and
  `publish-npm` (`tools/build-npm-package.sh`, then `npm publish`), both
  gated on their token through the workflow-level `env`, because a job-level
  `if:` cannot read `secrets`. The secret is `PYPI_API_TOKEN`, not `TWINE`;
  Phase 11's wording is corrected below.)*
- [x] `README.md` presents `pip install tpt-nrg` and `npm install tpt-nrg`
  as available today, and carries PyPI/npm/crates.io/Codecov/docs.rs badges,
  but nothing has ever been published to any of those registries (no tag
  pushed yet). Soften these to a "planned" note until a first successful
  publish, so a new user's first action doesn't 404. *(fixed: the registry
  badges are gone, and the install section leads with a status note and the
  commands that work today (a checkout, `cargo install --path`, the
  container image); the registry forms are shown as what they become "once a
  release is published". `tools/check-badges.py` in CI keeps it that way. The
  PyPI badge was also pointing at `pypi.org` while rendering a `crates.io`
  image, and the npm badge named `tpt-nrg` where the package is
  `tpt-nrg-wasm`.)*

### Automation
- [x] Adopt `release-plz` (or `cargo-smart-release`) for cross-crate
  changelog/version bumps *(carried over from Phase 11; also would have
  caught the `release.yml` bug above since it manages publishing itself)*
  *(`release-plz.toml` plus `.github/workflows/release-plz.yml`, which only
  ever runs `release-plz release --pr` and then the packaging pre-flight.
  Publishing deliberately stays tag-driven in `release.yml`: one place owns
  crates.io, PyPI, npm, the image, and the GitHub release, and a human stays
  between "the code is ready" and "the artifacts are public".)*
- [x] Add a `cargo publish --dry-run` check per crate to CI on any PR that
  touches a `Cargo.toml`, so a release-workflow regression is caught before
  a real tag push *(the `publish-check` job, gated on a `paths:` filter, runs
  `tools/publish-dry-run.sh`: `cargo package --list` per crate, then
  `cargo publish --dry-run`. Before the first release the dry run cannot
  resolve a workspace dependency that is not on crates.io yet, so exactly
  that failure is reported as a warning naming the missing crate and the run
  continues; every other failure fails the job. The check stops warning by
  itself once the first tag is published.)*
- [x] Add a badge-honesty check (manual pass or lightweight CI) confirming
  every `README.md` badge points at a registry entry that actually exists
  *(`tools/check-badges.py`, run by the `badges` CI job. Registry targets
  (crates.io, PyPI, npm, docs.rs) must resolve or be listed in
  `.github/badge-allowlist.txt` with a reason; other targets are reported as
  warnings, because a private repository answers 404 to an anonymous
  request. `--offline` lists what would be checked.)*

### Adoption / usability
- [x] `cargo generate` project template *(carried over from Phase 11 —
  scaffolded `Cargo.toml`, example `system.json`, `main.rs` that loads +
  solves it)* *(`templates/energy-system/`, usable with `cargo generate`, and
  `tpt-nrg new <NAME>` renders the same embedded files, so the two paths
  cannot drift. `--local` rewrites the dependencies to `path` entries, which
  is what makes a generated project buildable before the first release; the
  CLI validates the bundled case before writing anything, and the `cli` CI
  job generates a project and runs it.)*
- [x] Add a Rust-free "5-minute quickstart" to the README/book: public
  MATPOWER case → `tpt-nrg convert` → `tpt-nrg run` → `tpt-nrg viz`, using
  only the CLI, for adopters who are grid engineers rather than Rust
  developers *(a "Five-minute quickstart (no Rust)" section in `README.md`
  and `docs/book/src/cli-quickstart.md`, which also covers the study flags,
  the round-trip check, and the exit-code contract)*
- [x] Docker image for the CLI (`FROM scratch` + the static binary already
  built per-OS in `release.yml`'s `cli-binaries` job) *(`Dockerfile`: a
  multi-stage build on `rust:1-alpine` with `musl-dev`, a static
  `x86_64-unknown-linux-musl` binary (overridable with `--build-arg
  TARGET=`), and a `FROM scratch` runtime. `release.yml` gained a `docker`
  job that pushes `ghcr.io/tpt-solutions/tpt-energy:<tag>` and `:latest` with
  the buildx cache. It builds the binary in the image rather than reusing the
  `cli-binaries` artefacts, because those are Linux-only and this has to work
  on a developer's machine too; the image is therefore only exercised by CI,
  not by a local run here.)*
- [x] Golden-case gallery: document the existing IEEE 14/30/57-bus golden
  test cases in the book as a table of one-line `tpt-nrg run` commands
  *(`docs/book/src/golden-cases.md`, with the converged values measured from
  the committed cases, plus the fault, dispatch, resource, and storage
  fixtures and how to regenerate them. Linked from `README.md` and the book
  `SUMMARY.md`.)*

### Innovation
- [x] Hosted browser playground *(carried over from Phase 11 — mdBook +
  WASM + `tpt-nrg-viz`; the pieces (`wasm_visualize_json`,
  `wasm_economic_dispatch_json`) already exist)*; link it from the README
  above "Install" once live, since it's the fastest path to a user
  experiencing the library before installing anything *(`playground/`:
  `index.html`, `styles.css`, `app.js`, no framework and no build step
  beyond producing `pkg/`. `.github/workflows/playground.yml` builds the
  package, assembles the directory, runs `playground/tests/smoke.mjs`
  against the same WebAssembly module from Node, and deploys to GitHub
  Pages. The README section above "Install" describes running it locally
  rather than linking a URL that only exists after the first Pages deploy.)*
- [x] npm package for `tpt-nrg-wasm` with generated `.d.ts` *(carried over
  from Phase 11)* *(`crates/core/tpt-nrg-wasm/npm/`: the `package.json`
  `wasm-pack` does not generate, a checked-in `tpt_nrg_wasm.d.ts`, and a
  registry README. This required adding `crate-type = ["cdylib", "rlib"]` to
  the crate; without the `cdylib`, `wasm-pack` refused to build it at all.
  `tools/build-npm-package.sh` builds the package and `--check` fails when
  the checked-in declarations are stale; the `npm` CI job runs both, and
  `release.yml` publishes from a tag when `NPM_TOKEN` is set.)*
- [x] `tpt-nrg convert --diff a.raw b.m` — round-trip diff mode for format
  migration validation, reusing the existing `from_text`/`to_text` API in
  `tpt-nrg-interop` *(implemented in the library rather than the CLI, as
  `tpt_nrg_interop::diff`: `diff_systems`, `diff_text`, `diff_values`, and
  `round_trip`, with `Difference { kind, path, left, right }`. Records are
  matched by `id` instead of by position, because no two formats preserve
  record order the same way, and floats compare with a tolerance so a value
  that went through a fixed-width text field is not reported as changed. The
  CLI adds `--diff <FILE>`, `--against-from`, `--tolerance`, and
  `--format json`, plus `--round-trip` for the single-file case, and exits
  `1` when anything differs. In the library means the WASM and Python
  bindings can reuse the same rules.)*
