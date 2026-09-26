# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `DerAsset` enum (solar / wind / battery / load / diesel) with signed `net_power_mw()`.
- `MicrogridController` aggregation state with generation / demand / net-balance totals.
- `ControlStrategy` (`GridFollowing`, `GridForming`, `DroopControl`).
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
