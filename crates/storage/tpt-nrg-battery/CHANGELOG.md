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
- Add IEEE-style golden tests across dispatch, fault, resource, and storage crates
- Add substrate modules, IEEE golden tests, and example tooling
- Update gitignore and add benches, examples, RFCS, and docs
- Scaffold Rust workspace with tpt-nrg crates and project tooling

### Added
- `BatteryStorage` with SoC tracking, power/energy ratings, and round-trip efficiency split across the charge/discharge legs.
- `charge()` / `discharge()` with typed `BatteryError` results instead of silent clamping.
- `DegradationModel` (cycle life, calendar life, DoD curve, temperature derating) and `capacity_factor()`.
- Golden fixture `test-data/golden/storage/battery-soc-cycling.json`.
- `substrate` feature validating NMC-811 cell assumptions via `tpt-eng-materials`.
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

### Changed
- `charge` / `discharge` document their error contract via `# Errors` sections.

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
