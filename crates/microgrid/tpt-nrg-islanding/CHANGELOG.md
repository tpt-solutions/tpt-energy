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
- Update gitignore and add benches, examples, RFCS, and docs
- Scaffold Rust workspace with tpt-nrg crates and project tooling

### Added
- `IslandingDetector` with IEEE 1547 category II defaults (voltage / frequency / RoCoF, time window).
- `transition_to_island()` with load shedding, storage dispatch, and a new V/f reference.
- `resynchronize()` with phase / frequency / voltage matching and correct 2π wrapping.
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
