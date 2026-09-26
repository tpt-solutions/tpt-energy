# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `Electrolyzer` (electricity → H₂) and `FuelCell` (H₂ → electricity) specific-energy models.
- `HydrogenSystem` tank-coupled charge/discharge with round-trip efficiency accounting.
- Golden fixture `test-data/golden/storage/hydrogen-efficiency.json`.
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
