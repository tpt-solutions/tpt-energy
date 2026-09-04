# RFC 0004 — Hydrogen Electrolysis Round-Trip Efficiency

| Field         | Value                                     |
|---------------|-------------------------------------------|
| Status        | Proposed                                  |
| Author        | TPT Energy maintainers                    |
| Created       | 2026-09-04                                |

## Summary

Extend `tpt-nrg-hydrogen` to model temperature- and partial-load-dependent
efficiency curves, plus tank boil-off, so the round-trip efficiency
figure is meaningful for system planning rather than an idealised 34%.

## Motivation

The current crate uses a constant specific-energy assumption:

- Electrolyzer: 50 kWh/kg H₂ (constant).
- Fuel cell: 17 kWh/kg H₂ (constant).
- Tank: ideal (no boil-off).

Real systems have:

- Electrolyzer specific consumption rising at part load (down to ~120%
  of nominal at 20% load).
- Fuel cell efficiency falling at part load (down to ~80% of nominal).
- Compressed H₂ tanks losing 0.1–1% per day to boil-off, depending on
  insulation and tank size.
- PEM stacks degrading over time (5–10% over 40,000 hours).

Without modelling these, dispatch optimisations over- or under-estimate
the value of H₂ storage for grid balancing.

## Design

### Curve-based efficiency

```rust
pub struct Electrolyzer {
    pub spec_energy_curve: EfficiencyCurve,   // (load_fraction -> kWh/kg)
    pub power_rating_mw: f64,
    pub stack_age_hours: f64,                 // affects effective area
}
```

`EfficiencyCurve` is a piecewise-linear or polynomial representation of
specific consumption vs load fraction.

### Tank boil-off

```rust
pub struct HydrogenTank {
    pub capacity_kg: f64,
    pub current_h2_kg: f64,
    pub boil_off_fraction_per_day: f64,       // 0.001 typical
}
```

Applied at each `step()` boundary.

### Degradation

A simple `DegradationModel` analogous to the battery crate: linear
efficiency derate with hours of operation.

## Drawbacks

- Adds complexity for the common-case planning study that just needs a
  round-trip figure.
- Requires benchmark data from real electrolyzer / fuel-cell specs to
  populate default curves; we ship sensible defaults but they should
  be re-validated per project.

## Alternatives

- Keep the constant-efficiency model and document it as "ideal PEM,
  50/17 kWh/kg" so users with higher-fidelity requirements plug in
  their own data.

## Open Questions

- Should we model dynamic H₂ demand-response (electrolyzer as a
  controllable load) explicitly in `tpt-nrg-der`, or keep the H₂ crate
  decoupled?
- Does RFC 0005 (VPP) need to know H₂ round-trip efficiency at the
  asset level? If so, this crate should expose `effective_efficiency()`
  as part of the public API.

## Adoption

When accepted:

1. Refactor `Electrolyzer` and `FuelCell` to hold an `EfficiencyCurve`.
2. Add `HydrogenTank` and migrate `current_hydrogen_kg` to live there.
3. Update `HydrogenSystem::round_trip_efficiency()` to use the curve
   at the operating load rather than the constant.
4. Update `examples/battery-arbitrage.rs` or add a sibling
   `hydrogen-arbitrage.rs` example.
