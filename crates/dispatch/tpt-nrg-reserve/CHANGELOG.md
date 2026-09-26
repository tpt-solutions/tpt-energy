# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tpt-solutions/tpt-energy/releases/tag/v0.1.0) - 2026-09-26

### Other

- Add per-crate docs, move benches into crates, and update golden test data
- Fix correctness bugs across power flow, dispatch, and resource crates
- Update gitignore and add benches, examples, RFCS, and docs
- Scaffold Rust workspace with tpt-nrg crates and project tooling

### Added
- `spinning_reserve_margin()` headroom calculation (pure availability; load is not subtracted).
- `contingency_reserve_requirement()` NERC-style N-1 + load-fraction requirement.
- `assess_reserves()` combined `ReserveAssessment` with signed margin.
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

### Changed
- `spinning_reserve_margin` now takes `(online_capacity_mw, current_output_mw)` — the load argument and the removed `total_operating_reserve` are superseded by `assess_reserves` (see the workspace `CHANGELOG.md` for the full API history).

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
