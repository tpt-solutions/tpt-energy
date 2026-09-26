# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `NetworkTopology` adjacency-list construction from an `EnergySystem` (non-contiguous bus ids supported).
- `find_islands()` connected-component analysis and `find_shortest_path()` BFS path search.
- `AdmittanceMatrixBuilder` Y-bus construction (lines, taps, phase shifters) with dense storage and a sparse COO export.
- `substrate` feature wiring the `tpt-math-linalg-sparse` sparse path.
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

### Changed
- `#[must_use]` annotations on pure methods.

### Fixed
- Removed an unused internal helper and import.

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
