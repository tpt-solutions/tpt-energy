# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `FaultAnalyzer` with slack-based Thevenin impedance search (radial R/X accumulation).
- `SequenceNetwork` construction with `Z2 = Z1`, `Z0 = 3·Z1` defaults and explicit override.
- `calculate_fault_current()` for three-phase, line-to-line, line-to-ground, and double-line-to-ground faults.
- Textbook validation (Glover, Sarma & Overbye, Example 7.5).
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

### Changed
- `#[must_use]` annotations on pure methods.

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
