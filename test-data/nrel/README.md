# NREL test fixtures

No NREL-derived datasets are currently committed to this repository.
The solar-position and PV-output golden fixtures that `tpt-nrg-solar`
validates against live in [`test-data/golden/solar/`](../golden/solar/):

- `nrel-spa-zenith.json` - reference sun-position samples for the
  crate's SPA-equivalent algorithm (Meeus/NOAA-style, ~+/-0.01 deg).
- `pv-output-derating.json` - DC output at standard test conditions with
  thermal derating and inverter clipping.

To add real NREL data (e.g. SPA reference output or NSRDB measurement
series), commit it here as JSON/CSV and extend the golden tests in
`crates/resource/tpt-nrg-solar/tests/` to read it. Keep files small
(<1 MB) and record the source, site, and license in a header field.
