# `tpt-nrg-cli`

The `tpt-nrg` binary: solve a case, convert it between formats, or draw it,
without writing any Rust.

```sh
cargo install tpt-nrg-cli
```

Prebuilt binaries for Linux (x86-64, aarch64), macOS (aarch64), and Windows
(x86-64) are attached to every GitHub release.

## `tpt-nrg run`

```sh
tpt-nrg run --system test-data/ieee/ieee14.json --method newton-raphson
```

```
System      : IEEE 14-Bus Test Case (ieee14)
Method      : newton-raphson  -> converged=true after 7 iterations (mismatch 3.011e-12)
Losses      : 3.771 MW, -6.239 MVAr

   BUS  TYPE         |V| pu   angle deg        GEN MW
----------------------------------------------------
     1  slack        1.0600       0.000         55.67
     2  pv           1.0450      -0.437         80.00
...

BRANCH  NAME                  P MW       Q MVAr     LOAD %
----------------------------------------------------------
     1  1-2                 20.857       17.128      13.5%
```

`--format json` emits the same data as a machine-readable document.

| Flag                    | Effect                                              |
|-------------------------|-----------------------------------------------------|
| `--system`, `-s`        | the case to analyse (required)                      |
| `--from`                | input format; defaults to `json`                    |
| `--method`, `-m`        | `newton-raphson` (default), `gauss-seidel`, `fast-decoupled`, `dc` |
| `--format`, `-f`        | `table` (default) or `json`                         |
| `--tolerance`           | convergence tolerance in per-unit; default `1e-6`   |
| `--max-iterations`      | iteration cap; default `50`                         |
| `--dispatch <MW>`       | run an economic dispatch instead of a power flow    |
| `--commit`              | run a 24-hour priority-list unit commitment        |
| `--fault-bus <ID>`      | run a short-circuit study at a bus                 |
| `--lcoe-capex <DOLLAR>` | run an LCOE calculation (needs `--annual-energy-mwh`) |

### Dispatch needs cost curves

`--dispatch` follows a merit order, which comes from each generator's cost
curve. A case whose generators have no curve has no merit order, so the CLI
says so rather than reporting a confusing capacity error:

```
$ tpt-nrg run --system ieee14.json --dispatch 100
tpt-nrg: --dispatch needs cost curves: none of this system's generators has
one, so there is no merit order to dispatch along. Add a `cost_curve` to a
generator, or use the native JSON/YAML format where cost curves can be
written.
```

For the same reason, `--commit` prints a note when it cannot price the
schedule rather than a bare `$0.00`.

## `tpt-nrg convert`

```sh
tpt-nrg convert --from matpower --to json case14.m -o case14.json
```

Omit `-o` to write to standard output. `--from` and `--to` accept `json`,
`yaml`, `csv`, `matpower`, `psse`, and `cim` — the same set as
[`tpt-nrg-interop`](crate-interop.md).

## `tpt-nrg viz`

```sh
tpt-nrg viz --system case.json -o diagram.svg
```

Solves the case and writes the SVG single-line diagram and heatmap from
[`tpt-nrg-viz`](crate-viz.md). `--no-legend` and `--no-flow-labels` trim the
overlays.

## Exit codes

| Code | Meaning                                                    |
|------|------------------------------------------------------------|
| `0`  | success                                                    |
| `1`  | the analysis ran but produced no result (non-convergence, an infeasible problem) |
| `2`  | bad input or usage: an unreadable file, an unknown format, a missing bus, a missing flag |

This split follows the `ErrorKind` scheme proposed in
[RFC 0006](https://github.com/tpt-solutions/tpt-energy/blob/master/rfcs/0006-unify-error-handling.md),
so a script can tell "the case is bad" from "the analysis failed" without
parsing stderr.

## Testing

```sh
cargo test -p tpt-nrg-cli
```

The CI `cli` job additionally runs every subcommand against the committed
IEEE 14-bus case and asserts that a bad input exits non-zero.
