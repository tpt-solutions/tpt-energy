# tpt-nrg-interop

Import and export `EnergySystem` models in the formats power-systems
engineers actually exchange: **MATPOWER** (two-way), **PSS/E RAW** (two-way),
**CIM / IEC 61970 RDF/XML** (two-way), **YAML**, and **CSV** (flat or
bundled), alongside the native JSON.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy) — power
systems, resource modeling, storage, dispatch, and grid economics in pure
Rust.

## Features

- One `Format` enum and one `from_text` / `to_text` pair, so calling code
  does not branch on the format.
- Every converter validates its output with `EnergySystem::validate`, so a
  successful return always means a structurally sound system.
- `InteropErrorKind` — a stable slug (`parse`, `bad_value`,
  `missing_record`, `invalid_system`, `unsupported`, `io`) a CLI exit code, a
  WASM boundary, or a language binding can branch on.
- PSS/E dialect (v29–v33) is inferred from the record shape rather than
  declared, and Fortran `D` exponents are accepted.
- MATPOWER write-then-read is byte-identical, so a case can be regenerated
  without a diff.
- CIM import and export are symmetric, including the slack bus.
- `diff` compares two systems structurally: records matched by `id` rather
  than by position, floating-point fields compared with a tolerance, and
  every difference reported with its path (`buses[id=4].load_mw`).
  `round_trip` answers "is this writer lossless for this case?" in one call.

## Installation

```toml
[dependencies]
tpt-nrg-interop = "0.1"
```

## Usage

```rust
use tpt_nrg_core::EnergySystem;
use tpt_nrg_interop::{from_text, to_text, Format};

// One pair of functions, any format.
let system: EnergySystem = from_text(matpower_source, Format::Matpower)?;
let cim = to_text(&system, Format::Cim)?;
let yaml = to_text(&system, Format::Yaml)?;
# Ok::<(), tpt_nrg_interop::InteropError>(())
```

Branching on the failure:

```rust
use tpt_nrg_interop::{from_text, Format, InteropErrorKind};

match from_text(source, Format::Psse) {
    Ok(system) => println!("{} buses", system.buses.len()),
    Err(e) if e.kind() == InteropErrorKind::Parse => eprintln!("malformed RAW"),
    Err(e) => eprintln!("{e}"),
}
# Ok::<(), tpt_nrg_interop::InteropError>(())
```

Checking that a conversion did not lose anything:

```rust
use tpt_nrg_interop::{diff_text, round_trip, Format};

// Two files, in any two formats.
for difference in diff_text(&json_case, Format::Json, &matpower_case, Format::Matpower)? {
    println!("{}: {} -> {}", difference.path(), difference.left(), difference.right());
}

// Or one file through its own writer.
assert!(round_trip(&matpower_case, Format::Matpower)?.is_empty());
# Ok::<(), tpt_nrg_interop::InteropError>(())
```

## Crates.io metadata

- **Categories**: `science`, `parser-implementations`, `encoding`
- **Keywords**: `power-systems`, `matpower`, `psse`, `cim`, `iec-61970`,
  `interoperability`

## Status

**Alpha.** Every format round-trips against the committed IEEE 14/30/57
cases, but the CIM profile is a documented subset of IEC 61970 rather than
the whole standard, and the PSS/E reader takes the leading, positionally
stable fields of a record.

## Testing

```sh
cargo test -p tpt-nrg-interop
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
