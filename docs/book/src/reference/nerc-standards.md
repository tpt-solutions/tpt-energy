# NERC Standards Compliance Review

This document tracks TPT Energy's coverage of the North American
Electric Reliability Corporation (NERC) reliability standards most
relevant to the toolkit's scope: transmission planning, operations,
and resource adequacy.

> **Status: planning.** TPT Energy models are useful for compliance
> studies but do not (yet) produce signed-and-sealed compliance
> artefacts. This document is a gap analysis to guide that work.

---

## Resource adequacy & planning

| Standard | Title | TPT Energy coverage |
|----------|-------|--------------------|
| TPL-001 | Transmission system planning performance | ✓ `tpt-nrg-powerflow` AC/DC, `tpt-nrg-fault`, `tpt-nrg-state-estimation` |
| TPL-007 | Geomagnetic disturbance | ✗ |
| MOD-031 | Demand and energy data | Partial — `tpt-nrg-load` provides forecasting primitives |

## Operations

| Standard | Title | TPT Energy coverage |
|----------|-------|--------------------|
| TOP-001 | Real-time operations | ✓ `tpt-nrg-der` controller + `tpt-nrg-wasm` for edge / control-room dispatch |
| TOP-002 | Monitoring by RC/BA | ✓ `tpt-nrg-state-estimation` |
| TOP-003 | Outage coordination | ✗ |
| IRO-006 | Reliability coordination — outage scheduling | ✗ |

## Frequency & voltage support

| Standard | Title | TPT Energy coverage |
|----------|-------|--------------------|
| VAR-001 | Voltage and reactive control | Partial — `tpt-nrg-powerflow` enforces voltage setpoints at PV buses; Q-limit enforcement is tracked for a future release |
| BAL-001 | Real power balancing | ✓ `tpt-nrg-economic-dispatch` + `tpt-nrg-unit-commitment` |
| BAL-002 | Disturbance control — spinning reserve | ✓ `tpt-nrg-reserve` spinning/contingency reserve |
| BAL-003 | Frequency response | ✓ Indirectly via `tpt-nrg-islanding` RoCoF detection |
| PRC-005 | Protection system maintenance | ✗ |
| PRC-006 | Underfrequency load shedding | ✓ Indirectly via `tpt-nrg-islanding` UV detection |
| PRC-024 | Generator frequency / voltage ride-through | ✗ |

## Modelling & validation

| Standard | Title | TPT Energy coverage |
|----------|-------|--------------------|
| MOD-026 | Verification of generator models | ✗ |
| MOD-027 | Verification of turbine-governor and load-control models | ✗ |
| MOD-032 | Data for power system modelling | ✓ Implicit — `EnergySystem::from_json` accepts MATPOWER-style case files |
| MOD-033 | Steady-state and dynamic system model validation | Partial — golden tests against IEEE 14/30/57 |

## Path to compliance

1. **Ride-through curves** in `tpt-nrg-islanding` — extend the
   `IslandingDetector` defaults to include the IEEE 2800-2022
   voltage/frequency ride-through envelope.
2. **Q-limit enforcement** in `tpt-nrg-powerflow` — track for the
   next solver release; required by VAR-001-5.
3. **Outage scheduling** in a new `tpt-nrg-outage` crate —
   deconflict planned and forced outages against the network.
4. **Documentation** of compliance artefacts (PSDS, RAS settings,
   UFLS scheme) — defer to a `tpt-nrg-compliance` companion crate.

This is a multi-month effort and is deferred to a post-1.0 release.

---

## Related work

- FERC Order 2222 (DER aggregation): relevant for `tpt-nrg-vpp`.
- NERC IVGTF recommendations for inverter-based resources: relevant
  for the future grid-forming inverter modelling in `tpt-nrg-islanding`
  and `tpt-nrg-der`.
- IEEE 2800-2022 (IBR ride-through): see the
  `docs/book/src/reference/iec-standards.md` document.
