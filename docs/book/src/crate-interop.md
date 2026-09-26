# `tpt-nrg-interop`

Import and export `EnergySystem` models in the formats power-systems
engineers actually exchange:

| Format            | Module                | Two-way | Notes                                       |
|-------------------|-----------------------|---------|---------------------------------------------|
| `json`            | native                | yes     | the canonical form                         |
| `yaml`            | `tabular`             | yes     | the same `serde` schema as the JSON         |
| `csv`             | `tabular`             | yes     | flat (`record` column) or bundled directory |
| `matpower`        | `matpower`            | yes     | `case*.m` MATLAB scripts                    |
| `psse`            | `psse`                | yes     | RAW data files, v29–v33                     |
| `cim`             | `cim`                 | yes     | RDF/XML, IEC 61970 subset                   |

Everything is reachable through one pair of functions, so calling code does
not have to know which format it is handling:

```rust
use tpt_nrg_interop::{from_text, to_text, Format};

let system = from_text(source, Format::Matpower)?;
let cim     = to_text(&system, Format::Cim)?;
# Ok::<(), tpt_nrg_interop::InteropError>(())
```

## Errors

Every failure carries an `InteropErrorKind` — a stable slug a CLI exit code,
a WASM boundary, or a language binding can branch on:

| Kind              | Meaning                                          |
|-------------------|--------------------------------------------------|
| `missing_record`  | a required block, record, or field was absent   |
| `bad_value`       | a field could not be interpreted                |
| `parse`           | the input is not valid in that format            |
| `invalid_system`  | it parsed, but is not a structurally valid system |
| `unsupported`     | the format is not implemented                    |
| `io`              | a file could not be read                         |

```rust
use tpt_nrg_interop::{from_text, Format, InteropErrorKind};

let err = from_text("not a case", Format::Matpower).unwrap_err();
assert_eq!(err.kind(), InteropErrorKind::Parse);
# Ok::<(), tpt_nrg_interop::InteropError>(())
```

Every converter validates its output with `EnergySystem::validate`, so a
successful return always means a structurally sound system.

## MATPOWER

The importer is textual, not a MATLAB interpreter: it understands
`mpc.<name> = <matrix|number|string>;` assignments, which covers every
published `case*.m` file. Comments start at `%`, matrices may span lines, and
`mpc.baseMVA` is required.

The system identifier comes from the enclosing `function mpc = <name>` line,
because in MATLAB the function name *is* the case identity; `mpc.caseName`
supplies the human-readable name. That split is what makes a
write-then-read cycle byte-identical:

```rust
use tpt_nrg_core::EnergySystem;
use tpt_nrg_interop::matpower;

let sys = EnergySystem::from_json(include_str!("../../test-data/ieee/ieee14.json"))?;
let text = matpower::to_matpower(&sys)?;
let reparsed = matpower::from_matpower(&text)?;

// A second round trip is byte-identical.
assert_eq!(matpower::to_matpower(&reparsed)?, text);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## PSS/E

RAW files wrap records at column 80 and delimit them with `/`, so a record
can span lines and a line can hold several records. The parser tracks
sub-blocks rather than physical lines.

The number of sub-blocks per record is what identifies the dialect — PSS/E
only ever *appends* to a record — so the count is inferred rather than
declared, trying the narrowest candidate first and requiring each record's
leading sub-block to look right:

| Dialect | Sub-blocks per bus record |
|---------|---------------------------|
| v29     | 3                         |
| v30     | 4                         |
| v32     | 5                         |
| v33     | 6                         |

Fortran `D` exponents (`1.5D+02`) are accepted everywhere. Leading,
positionally stable fields are read; trailing fields this version does not
need are ignored.

## CIM / IEC 61970

CIM exchange files are RDF/XML. This crate implements the subset that
carries a steady-state network model, reading by *local* name so any
namespace prefix (`cim:`, `md:`, or none) works.

The mapping onto the TPT Energy model is a documented choice, not an
identity:

| TPT Energy | CIM                                                        |
|------------|------------------------------------------------------------|
| `Bus`      | a `ConnectivityNode` reached from a `BusbarSection`         |
| `Branch`   | an in-service `ACLineSegment`; `PowerTransformer` is imported as a branch with a unity tap unless a `TransformerEnd.ratio` is present |
| `Generator`| a `RotatingMachine` or `EquivalentInjection`                 |
| bus load   | summed `EnergyConsumer.p` / `.q` attached to the node         |
| slack bus  | the node holding an `EquivalentInjection`                    |

A slack bus with no machine gets a synthetic `EquivalentInjection` on
export, so the document round-trips.

## CSV

Two shapes, because both are useful:

**Flat** — one table with a `record` column selecting the row type. This is
what [`to_flat_csv`](https://docs.rs/tpt-nrg-interop) writes and
[`from_flat_csv`](https://docs.rs/tpt-nrg-interop) reads:

```csv
record,id,name,type,base_kv,load_mw
bus,1,Slack,Slack,132,0
bus,2,Load,Pq,132,50
```

**Bundled** — a directory with one table per record type
(`buses.csv`, `branches.csv`, `generators.csv`, `loads.csv`, `storage.csv`,
`system.csv`). This is the shape a case handed to you tends to arrive in.

Column names match the JSON schema, are matched case- and
space-insensitively, and an empty cell means "use the default".

## Testing

```sh
cargo test -p tpt-nrg-interop
```

The suite round-trips every format, including the committed IEEE 14/30/57
cases, and asserts that identifiers and in-service flags survive.
