# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `SolarModel::solar_position()` — SPA-equivalent sun position (Meeus/NOAA-style, ≈ ±0.01°) with refraction and Kasten–Young air mass.
- `clear_sky_irradiance()` — Ineichen GHI/DNI/DHI with altitude correction.
- `plane_of_array_irradiance()` — isotropic-sky transposition.
- `PvPlant` — NOCT cell temperature, temperature derating, soiling, DC/AC clipping, inverter efficiency.
- Golden tests: `nrel-spa-zenith.json` and `pv-output-derating.json`.
- `substrate` feature validating Earth–Sun distance handling via `tpt-sci-astro`.
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

### Changed
- `#[must_use]` annotations on pure methods and builders.

### Fixed
- Azimuth is now finite and reported as north when the sun is within float-epsilon of the zenith (previously `NaN` at tropical noon).
- Ineichen DNI clamping uses `clamp` instead of nested `max`/`min`.
- Julian-day computation avoids potential-precision-loss casts.

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
