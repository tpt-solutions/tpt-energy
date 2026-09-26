# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `economic_dispatch()` lossless merit-order dispatch with λ iteration and must-run handling.
- `storage_arbitrage()` price-forecast scheduler producing `ArbitragePlan`.
- Typed `DispatchError` for infeasible loads (no generators, above capacity, below must-run).
- Golden fixture `test-data/golden/dispatch/economic-dispatch-5gen.json`.
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

### Changed
- `storage_arbitrage` sorts prices with a total ordering (no `unwrap` on partial comparisons).

### Fixed
- Arbitrage iteration indexes prices directly from the slice (no off-by-one risk from manual index loops).

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
