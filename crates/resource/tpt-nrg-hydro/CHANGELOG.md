# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `HydroPlant` head × flow × efficiency power calculation with max-flow clamping.
- Minimum-flow shutdown semantics: below `min_flow_m3s` the unit produces nothing.
- Integration with `GeneratorType::Hydro` in `tpt-nrg-core`.
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
