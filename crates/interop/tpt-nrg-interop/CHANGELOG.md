# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

## License
Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
