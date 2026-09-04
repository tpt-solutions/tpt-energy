# RFC 0002 — Battery Degradation Integration

| Field         | Value                                     |
|---------------|-------------------------------------------|
| Status        | Proposed                                  |
| Author        | TPT Energy maintainers                    |
| Created       | 2026-09-04                                |

## Summary

Wire `tpt-materials::BatteryDegradationModel` capacity-fade curves
into `tpt-nrg-battery::DegradationModel` so that real chemistry-specific
curves (LFP, NMC, NCA) can drive the SoC-aware dispatch and LCOE
calculation.

## Motivation

The current `DegradationModel::li_ion()` is a generic placeholder.
Different chemistries have very different cycle life, calendar life,
and DoD-vs-cycle curves; planners need to use the right curve for the
project's battery choice.

## Design

- Add an optional `chem` field to `DegradationModel` enum with
  variants `LiFePO4`, `NMC811`, `NCA`, `LeadAcid`, `VanadiumRedox`.
- When `tpt-substrate` is enabled, replace the defaults with
  curves from `tpt-materials`.
- Expose a builder API:
  `DegradationModel::from_material_curve(material: &Material)`.

## Drawbacks

- Need to keep the placeholder defaults in sync with the substrate
  models once they're available.

## Alternatives

- Ship chemistry-specific models in `tpt-nrg-battery` itself.
- Out of scope: leave the integration to the caller.

## Open Questions

- Should temperature derating use a single Arrhenius factor or
  per-chemistry activation energy?
