# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `render`, a dependency-free SVG single-line diagram: bus markers filled by
  voltage band, branch lines coloured by loading, per-branch MW labels, a
  slack-bus ring, and a two-row legend.
- `VizOptions` for the title, bus labels, flow labels, legend, generator
  markers, and background.
- `layout::compute`, a deterministic breadth-first grid layout keyed on
  shortest-path depth from the slack bus, so the output is reproducible and
  diffable.
- `Band` (voltage) and `LoadingBand` (loading) with contiguous bounds,
  labels, and colours, exposed so callers can reuse the scales.
- `render_ieee14` example writing `target/ieee14.svg`.
- This crate-level `README.md` with crates.io `categories` and `keywords`
  metadata.

## License
Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
