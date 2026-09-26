# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tpt-solutions/tpt-energy/releases/tag/v0.1.0) - 2026-09-26

### Other

- Add per-crate docs, move benches into crates, and update golden test data
- Add .gitattributes for LF normalization and CI health audit notes
- Fix correctness bugs across power flow, dispatch, and resource crates
- Scaffold Rust workspace with tpt-nrg crates and project tooling

### Added
- `UniformTimeSeries` with integration, statistics, and step/linear resampling.
- `TimeSeries` for arbitrary timestamped samples with sorted-order validation and saturating linear interpolation.
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

### Changed
- `TimeSeriesError::InvalidStep` and `IncommensurateStep` now carry `i64` payloads (resample steps are integers) instead of `f64`.

### Fixed
- Resampling now rejects incommensurate steps instead of silently relabeling the sample grid.
- Removed potential-precision-loss casts from duration and count arithmetic (single documented conversion helpers).

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
