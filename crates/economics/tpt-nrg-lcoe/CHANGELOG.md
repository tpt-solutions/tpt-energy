# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `levelized_cost_of_energy()` discounted LCOE with capital-recovery factor and zero-rate limit.
- `net_present_value()` and bisection-based `internal_rate_of_return()`.
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

### Changed
- Year-index conversions are checked instead of raw casts.

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
