# RFC 0003 — Microgrid Islanding

| Field         | Value                                     |
|---------------|-------------------------------------------|
| Status        | Proposed                                  |
| Author        | TPT Energy maintainers                    |
| Created       | 2026-09-04                                |

## Summary

Define a deterministic state machine for microgrid operating modes
(grid-connected, islanded, resynchronising, blackout) and a unified
control interface for DER assets.

## Motivation

The current `tpt-nrg-islanding` crate implements detection thresholds
and a single transition function, but real microgrid controllers must
track operating mode over time and coordinate multiple DERs during
seamless transitions.

## Design

- Add `MicrogridState` enum: `GridConnected`, `Islanding`,
  `Islanded`, `Resynchronising`, `BlackStart`.
- Add a `MicrogridController::step(state, measurements) -> ControlAction`
  method that advances the state machine and emits per-asset setpoints.
- Use IEEE 1547-2018 categorisation for thresholds and timing.

## Drawbacks

- Adds a non-trivial state machine that must be carefully tested.

## Alternatives

- Keep the current threshold-based approach; the user composes the
  state machine externally.

## Open Questions

- Should we model the sub-cycle transients of the grid-forming
  inverter, or stay in the steady-state envelope?
