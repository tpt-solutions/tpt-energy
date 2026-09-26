# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tpt-solutions/tpt-energy/releases/tag/v0.1.0) - 2026-09-26

### Other

- Add per-crate docs, move benches into crates, and update golden test data
- Add .gitattributes for LF normalization and CI health audit notes
- Scaffold Rust workspace with tpt-nrg crates and project tooling

### Added
- `EnergySystem` container with buses, branches, generators, loads, storage, and metadata.
- `Bus` with `BusType` (`Slack` / `PV` / `PQ` / `Isolated`), `Branch` with tap and phase-shift modeling, `Generator` with `P`/`Q` limits and optional cost / heat-rate / power curves.
- JSON (de)serialization (`from_json`, `from_json_file`, `to_json_pretty`) with permissive defaults.
- Structural validation: reference integrity, duplicate-id rejection, exactly-one-slack enforcement.
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

### Changed
- `#[must_use]` annotations on pure accessors and builder methods across the public API.
- `# Errors` documentation sections on all `Result`-returning public functions.

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
