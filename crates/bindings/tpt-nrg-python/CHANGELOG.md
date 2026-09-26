# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- The `tpt_nrg` extension module, built with `maturin` and using the
  `abi3-py39` ABI so one wheel covers Python 3.9 and later.
- `System`: `from_file` (format inferred from the extension), `from_json`,
  `from_text`, `power_flow`, `economic_dispatch`, `unit_commitment`,
  `fault_current_pu`, `carbon_intensity`, `lcoe`, `to_svg`, `to_dict`, and a
  `to_<format>` method per exchange format.
- `PowerFlowResult` and `LcoeResult` result types.
- `PowerFlowMethod` enum with a `name` getter matching the CLI and
  `Format.parse`.
- `EnergyError` (a `ValueError` subclass) carrying a stable kind slug, plus
  `error_kind` to read it back, `formats`, and `validate`.
- `pyproject.toml` for `maturin` builds and PyPI metadata, and a Python API
  smoke test run in CI.
- This crate-level `README.md` with usage, an error table, and status.

## License
Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
