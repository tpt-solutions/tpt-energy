# RFC 0006 — Unify Error Handling Across the Workspace

| Field         | Value                                     |
|---------------|-------------------------------------------|
| Status        | Proposed                                  |
| Author        | TPT Energy maintainers                    |
| Created       | 2026-09-26                                |

## Summary

Replace the seven independent `thiserror` taxonomies in the workspace with a
single layered scheme: a `tpt-nrg-core::CoreError` root, one `ErrorKind`
discriminant shared by every crate, and per-crate enums that wrap `CoreError`
and keep their crate-specific detail. `tpt-nrg-error` becomes the
authoritative source of `ErrorKind`.

## Motivation

The workspace currently rolls an independent error enum per crate:

| Crate                        | Enum              | Variants |
|------------------------------|-------------------|----------|
| `tpt-nrg-core`               | `CoreError`       | 10       |
| `tpt-nrg-powerflow`          | `PowerFlowError`  | 6        |
| `tpt-nrg-economic-dispatch`  | `DispatchError`   | 4        |
| `tpt-nrg-battery`            | `BatteryError`    | 3        |
| `tpt-nrg-topology`           | `TopologyError`   | 3        |
| `tpt-nrg-timeseries`         | `TimeSeriesError` | 3        |
| `tpt-nrg-wasm`               | `WasmError`       | 3        |

Three concrete costs follow from this:

1. **Callers cannot match across layers.** An application that loads a system
   (`CoreError`), solves it (`PowerFlowError`), and dispatches it
   (`DispatchError`) needs three `match` arms for what a user perceives as one
   failure: "this case file is not usable".
2. **Serialization across the WASM boundary is ad hoc.** `tpt-nrg-wasm`
   already had to invent its own `{kind, message}` tag purely because
   `PowerFlowError` cannot be serialized. Every new FFI surface repeats the
   problem.
3. **Error policy is duplicated.** `Io` exists in `CoreError`; `Json` exists
   in both `CoreError` and `WasmError`; `NoSlackBus` exists in both
   `CoreError` and `PowerFlowError`. Fixing a message fixes one copy only.

## Design

### Layer 1 — `ErrorKind`

A new crate `tpt-nrg-error` owns a single non-exhaustive discriminant:

```rust
#[non_exhaustive]
pub enum ErrorKind {
    NotFound,
    InvalidInput,
    Validation,
    Duplicate,
    Convergence,
    Capacity,
    Numeric,
    Parse,
    Io,
    Unsupported,
    Internal,
}
```

`ErrorKind` is `Copy + Eq + Hash + Serialize` and serializes as a lowercase
string (`"not_found"`, `"convergence"`, …). It is the stable, documented
surface that bindings, CLI exit codes, and metrics key on.

### Layer 2 — `CoreError`

`CoreError` stays in `tpt-nrg-core` (it cannot depend on a crate that depends
on it), gains `kind()` and `From<ErrorKind>`, and keeps its existing variants.
`Io` and `Json` are kept as `#[from]` shims so existing call sites are
unaffected.

### Layer 3 — per-crate enums wrap `CoreError`

Each crate enum gains a `Core(#[from] CoreError)` variant, so any
`CoreError` can propagate through any layer unchanged, plus a `kind()`
accessor:

```rust
pub enum PowerFlowError {
    Core(#[from] CoreError),
    MultipleSlackBuses(usize),
    NonConvergence { iterations: usize, mismatch: f64 },
    // ...
}

impl PowerFlowError {
    pub fn kind(&self) -> ErrorKind { /* ... */ }
    pub fn retryable(&self) -> bool { /* Convergence => true */ }
}
```

Crates with no crate-specific failures (`tpt-nrg-hydro`) drop their enum
entirely and alias `pub type HydroError = CoreError`.

### Layer 4 — FFI

`tpt-nrg-wasm::WasmError` becomes a thin projection:

```rust
#[derive(Serialize)]
pub struct WasmError {
    pub kind: ErrorKind,     // machine-readable
    pub message: String,     // human-readable
    pub retryable: bool,
}
```

JS callers switch on `kind`, not on message text.

## Drawbacks

- `tpt-nrg-error` adds a crate to the dependency graph, and therefore to the
  `release.yml` publish matrix.
- `#[non_exhaustive]` on `ErrorKind` forces downstream `match` arms to be
  wildcard-terminated, which is a (small) source-compat break for anyone
  matching exhaustively today.
- Wrapping `CoreError` into every crate enum changes the `Debug` output and
  the `Display` prefix of error messages (`json error: ...` becomes
  `core: json error: ...`). Golden tests and doc examples that assert on
  message text must be updated.

## Alternatives

1. **Do nothing.** Each crate keeps its own taxonomy. Cheapest, and the status
   quo. Rejected because the WASM and CLI surfaces both need a single
   machine-readable discriminant, and both need it *now*.
2. **Collapse everything into one `tpt-nrg-error` enum.** Maximum
   uniformity, but `tpt-nrg-core` would depend on `tpt-nrg-error` and the
   split would be inverted, or `tpt-nrg-error` would pull in `tpt-nrg-core`
   for `EnergySystem`-typed variants. Rejected: it couples crates that have no
   other reason to be coupled.
3. **`snafu` instead of `thiserror`.** `snafu` generates context selectors
   that would make the wrapping ergonomic. Rejected for now: it is a hard
   dependency swap across 7 crates for ergonomics only, and `thiserror` is
   already in the allow-list.

## Open Questions

- Should `retryable()` live on `ErrorKind` rather than on each crate enum?
  Per-enum is more precise (a `Convergence` kind from a filter solve may not
  be retryable) but multiplies the surface.
- Does the CLI need per-`ErrorKind` exit codes, or is a single `2` for "user
  error" / `1` for "internal" enough for a first release?

## Adoption

When accepted:

1. Add `crates/core/tpt-nrg-error` with `ErrorKind`, `ResultExt`, and unit
   tests for the `kind()` mapping of every variant.
2. Give `CoreError` a `kind()`; add `Core(#[from] CoreError)` to
   `PowerFlowError`, `DispatchError`, `BatteryError`, `TopologyError`, and
   `TimeSeriesError`.
3. Alias away the enums that have no crate-specific variants.
4. Re-project `tpt-nrg-wasm::WasmError` onto `ErrorKind` and audit the
   `#[wasm_bindgen]` boundary for stringified errors.
5. Map `ErrorKind` to CLI exit codes in `tpt-nrg`.
6. Document the scheme in `docs/book/src/reference/error-handling.md` and
   add a `good first issue` for each crate whose `kind()` mapping is still
   uncovered.

