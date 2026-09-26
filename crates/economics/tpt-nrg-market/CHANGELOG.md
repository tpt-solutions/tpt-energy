# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `MarketSignal` price-statistics wrapper over `tpt-nrg-timeseries` with `MarketType`.
- `mean_price()`, `peak_price()`, `off_peak_price()` (10th percentile), `price_spread()`.
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

### Changed
- `off_peak_price` and `price_spread` return `Option<f64>` (`None` for an empty series).
- The percentile index is computed in integer arithmetic (no float casts).

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
