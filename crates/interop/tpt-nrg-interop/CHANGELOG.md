# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/tpt-solutions/tpt-energy/releases/tag/v0.1.0) - 2026-09-26

### Other

- Add project template, playground, npm packaging, and release automation
- Add per-crate docs, CI/release workflow updates, and new examples
- Distinguish parse/validation/infeasible error kinds in Python bindings
- Add Python bindings, CLI powerflow/dispatch/fault/viz commands, and interop visualization support
- Add CLI and interop crates, MATPOWER/PSS/E/CIM support, and topology fix

### Added
- Two-way MATPOWER (`matpower`) conversion of `case*.m` MATLAB scripts, with
  a byte-identical write-then-read round trip.
- Two-way PSS/E RAW conversion (`psse`), with the v29-v33 dialect inferred
  from the record shape and Fortran `D` exponents accepted.
- Two-way CIM / IEC 61970 RDF/XML conversion (`cim`), with a documented
  mapping and a small non-validating XML reader.
- Two-way YAML conversion (`tabular`), using the same `serde` schema as the
  native JSON.
- Two-way CSV conversion: a flat table with a `record` column, and a bundled
  directory of one table per record type (`tabular`).
- `Format`, `from_text`, and `to_text` so callers need not branch on format.
- `InteropError` with an `InteropErrorKind` discriminant
  (`missing_record`, `bad_value`, `parse`, `invalid_system`, `unsupported`,
  `io`).
- `field_index` and `format_number`, the two documented numeric conversions
  every format shares.
- This crate-level `README.md` with crates.io `categories` and `keywords`
  metadata.
- `diff` module: structural comparison of two energy systems
  (`diff_systems`, `diff_text`, `diff_values`, and `round_trip`) with
  `Difference` / `DifferenceKind`. Records are matched by `id` rather than by
  position, floating-point fields compare with a configurable tolerance
  (`DEFAULT_TOLERANCE`, or the `_with_tolerance` variants), and each
  difference carries a path such as `buses[id=4].load_mw`.
- `Format::from_path`, which infers a format from a file extension so a caller
  does not have to repeat it.

## License
Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
