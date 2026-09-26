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
- `tpt-nrg convert --diff <FILE>`: compare two cases field by field through
  `tpt_nrg_interop::diff`, exiting `1` when they differ. `--against-from`
  names the second case's format, `--tolerance` widens the floating-point
  comparison, and `--format json` emits the differences as a document.
- `tpt-nrg convert --round-trip`: parse a case, write it back in the same
  format, parse it again, and report what the writer lost.
- `tpt-nrg new <NAME>`: scaffold a study project from
  `templates/energy-system/`, with `--dir`, `--force`, and `--local` (point
  the dependencies at a checkout rather than at crates.io).
- Format inference from the file extension for `run`, `convert`, and `viz`,
  so `case.m` and `case14.json` need no `--from`; an unrecognised extension is
  a usage error rather than a silent fallback.

## License
Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
