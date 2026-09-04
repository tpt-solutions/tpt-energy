# IEC 61850 / IEC 61970 (CIM) Standards Compliance Review

This document tracks TPT Energy's coverage of the two most important
power-systems interoperability standards: **IEC 61850** (substation
automation and DER communication) and **IEC 61970 / CIM** (Energy
Management System common information model).

> **Status: planning.** TPT Energy currently uses its own
> `EnergySystem` data model, which is JSON-serialisable and maps
> directly to MATPOWER `.m` cases. Formal mapping to IEC 61850
> logical nodes and IEC 61970 / CIM packages is tracked for a later
> release; this document is a gap analysis to guide that work.

---

## IEC 61970 — Common Information Model (CIM)

The CIM defines the information exchanged between control-centre
applications (SCADA, EMS, market, planning). The relevant packages:

| CIM package | TPT Energy coverage |
|-------------|--------------------|
| `Wire.TopologicalNode` ↔ Bus | ✓ `Bus` (id, name, type, voltage pu) maps to `TopologicalNode` |
| `Wire.ConductingEquipment` ↔ Branch | ✓ `Branch` (from_bus, to_bus, r, x, b, rating) |
| `WiresGeneration.GeneratingUnit` ↔ Generator | ✓ `Generator` (id, bus, type, p/q schedule) |
| `WiresEnergyConsumer.EnergyConsumer` ↔ Load | ✓ `Load` (bus, p, q) |
| `OperationalLimits.CurrentLimit` | ✗ Branch ratings present but no per-element current limits |
| `WiresRegulatingControl.ShuntCompensator` | ✓ Bus `shunt_conductance_pu` / `shunt_susceptance_pu` |
| `WiresTapChanger` | ✗ Transformer tap ratio present on `Branch` but no tap-changer schedule |
| `Protection.ProtectionEquipment` | ✗ Relays in `tpt-nrg-protection` are not yet CIM-mapped |
| `Market.MarketParticipant` ↔ `tpt-nrg-market` | ✗ |
| `DER::DERGroup` ↔ `tpt-nrg-vpp` | ✗ |

**CIM RDF/XML serialiser**: not yet implemented. The JSON layout in
`test-data/ieee/*.json` is the current canonical exchange format.

---

## IEC 61850 — Substation Automation

IEC 61850 uses logical nodes (LN) grouped into Logical Devices (LD) per
IED (Intelligent Electronic Device). Coverage in TPT Energy:

| Logical Node | Role | TPT Energy coverage |
|--------------|------|--------------------|
| `MMXU` | Measurement (V, I, P, Q, f) | ✓ Implicit (read into state estimator) |
| `MSQI` | Sequence and imbalance | ✗ |
| `MMTR` | Metering (energy totals) | ✗ |
| `XCBR` | Circuit breaker | ✗ |
| `XSWI` | Switch / disconnect | ✗ |
| `YELT`, `YPSH`, `YPTR` | Tap changer / regulation | ✗ |
| `PTRC`, `PIOC`, `PDIF` | Protection functions | ✗ (`tpt-nrg-protection` models coordination but not LN mapping) |
| `DGEN` | DER generation controller | Partial — `tpt-nrg-der` covers Solar/Wind/Battery/Load/Diesel assets |
| `DRCT`, `DRCS` | DER controller / status | ✓ `MicrogridController` |

**MMS / Goose / Sampled Values**: out of scope (a transport concern,
not a data-model concern).

---

## Path to compliance

1. **CIM serialiser** (`tpt-nrg-cim` crate, future): implement
   `to_cim_xml(&EnergySystem) -> String` and `from_cim_xml(&str) ->
   EnergySystem` covering the `TopologicalNode`, `ConductingEquipment`,
   `GeneratingUnit`, `EnergyConsumer` packages.
2. **61850 LN mapping**: implement `from_ln_groups(&HashMap<LN, Value>)`
   in `tpt-nrg-protection` for `PTRC`, `PIOC`, `PDIF`.
3. **Compliance test**: validate against published
   IEC 61970-552 (CIMXML) and IEC 61850 SCL files from a real EMS.
4. **Round-trip tests**: parse → simulate → re-serialise → compare.

This is a multi-month effort and is deferred to a post-1.0 release.

---

## Related work

- IEC TR 61850-7-420 (DER logical nodes): relevant for
  `tpt-nrg-der` and `tpt-nrg-vpp` once a 61850 mapping crate exists.
- IEC 62325 (market): relevant for `tpt-nrg-market`.
- IEEE 1547 (DER interconnection): partly covered by
  `tpt-nrg-islanding`'s default thresholds.
