# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
