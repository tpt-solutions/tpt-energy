# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Priority-list / forward-dispatch heuristic with per-unit merit ordering.
- `unit_commitment()` returning the commitment matrix, outputs, and horizon cost.
- Golden fixture `test-data/golden/dispatch/unit-commitment-24hr.json`.
- `substrate` feature: upstream-validated continuous dispatch (`economic_dispatch_substrate`) via `tpt-math-optimize-general`.
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

### Changed
- Merit-order sorting uses a total float ordering (no `unwrap` on partial comparisons).

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
