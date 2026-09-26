# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `tpt-nrg run`: Newton-Raphson, Gauss-Seidel, fast-decoupled, and DC power
  flow, with `table` and `json` output and configurable tolerance and
  iteration cap.
- `tpt-nrg run --dispatch`, `--commit`, `--fault-bus`, and `--lcoe-capex`
  for economic dispatch, unit commitment, short circuit, and LCOE.
- `tpt-nrg convert --from <fmt> --to <fmt>` across all six exchange formats.
- `tpt-nrg viz` writing the SVG single-line diagram and heatmap.
- Exit codes 0 / 1 / 2 distinguishing success, a failed analysis, and bad
  input or usage.
- Actionable errors: a dispatch request against a case with no cost curves
  says so, and `--commit` notes when it cannot price the schedule.
- This crate-level `README.md` with crates.io `categories` and `keywords`
  metadata.

## License
Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
