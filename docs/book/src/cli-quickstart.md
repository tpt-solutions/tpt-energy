# Five-minute quickstart (CLI only)

No Rust project, no dependencies to wire up: the `tpt-nrg` binary reads the
formats power-systems engineers already exchange and writes the answers. This
page is the whole path from a public case file to a result you can look at.

If you would rather write code, start at [Quick Start](quick-start.md) instead.

## 0. Install the CLI

From a checkout, before the first crates.io release:

```sh
git clone https://github.com/tpt-solutions/tpt-energy
cd tpt-energy
cargo install --path crates/cli/tpt-nrg-cli
tpt-nrg --version
```

Or run it without installing, which is handy while trying it out:

```sh
cargo run -p tpt-nrg-cli -- --help
```

A container image is published to GitHub Container Registry on each release, so
no toolchain is needed at all:

```sh
docker run --rm -v "$PWD:/cases" ghcr.io/tpt-solutions/tpt-energy --help
```

## 1. Get a case

The IEEE test cases are public and in the public MATPOWER format:

```sh
curl -LO https://raw.githubusercontent.com/PowerAPI-Validation/Python-MATPOWER/master/powerdata/case14.m
```

Any of `case*.m` works. PSS/E RAW, CIM RDF/XML, YAML, CSV, and the native JSON
are equally valid inputs.

## 2. Solve it

```sh
tpt-nrg run --system case14.m
```

The format comes from the file extension, so `case14.m` needs no `--from`. The
output is the converged voltage profile, the branch flows, and the losses.

| Flag | Use it for |
|------|------------|
| `--method newton-raphson` | the default; full AC solution |
| `--method dc` | a fast first look at a large case |
| `--tolerance`, `--max-iterations` | loosening a case that will not converge |
| `--fault-bus 4` | a three-phase short-circuit study at that bus |
| `--dispatch 100` | economic dispatch against 100 MW of load (needs cost curves) |
| `--commit` | a 24-hour priority-list unit commitment |
| `--lcoe-capex 5e6 --annual-energy-mwh 90000` | a levelized-cost calculation |
| `--format json` | machine-readable output |

## 3. Convert it

```sh
tpt-nrg convert case14.m -o case14.json          # to the native JSON
tpt-nrg convert case14.json -o case14.m          # and back
tpt-nrg convert case14.m --to psse -o case14.raw # to PSS/E
tpt-nrg convert case14.json --to yaml -o case14.yaml
```

## 4. Check the conversion

This is the step that separates "it converted" from "it converted correctly".
A text format can only carry so much, and a case that loses a field silently is
worse than one that fails loudly.

```sh
tpt-nrg convert case14.m --round-trip
```

That parses the case, writes it back out in the same format, reads the result
again, and compares the two models field by field. Anything the writer dropped
or altered is printed with its path, and the exit code is `1`:

```
case14.m as matpower: 12 difference(s)
  removed  buses[id=3].name: "3" -> <absent>
  changed  branches[id=4].reactance_pu: 0.05917 -> 0.0592
```

To compare two cases directly, whatever their formats:

```sh
tpt-nrg convert case14.json --diff other-case.raw
tpt-nrg convert case14.json --diff other-case.m --against-from matpower
tpt-nrg convert case14.json --diff other-case.m --format json   # for scripting
```

The comparison is structural, not textual: records are matched by `id` rather
than by position, so a converter that reorders buses does not produce noise, and
floating-point fields compare with a tolerance, so a value that went through a
fixed-width text field is not reported as changed.

`--tolerance` widens that tolerance when a case was exported at low precision:

```sh
tpt-nrg convert case14.m --round-trip --tolerance 1e-6

## 5. Look at it

```sh
tpt-nrg viz --system case14.json -o case14.svg
```

`tpt-nrg-viz` draws a single-line diagram with the voltage profile and branch
loading as a heatmap, in plain SVG: no asset pipeline, no JavaScript. Open the
file in any browser, or embed it in a report.

## 6. Start a project

When the CLI has told you what you need, scaffold the Rust project that keeps
doing it:

```sh
tpt-nrg new my-study --local .    # --local points the dependencies at this checkout
cd my-study
cargo run
```

See [New project template](project-template.md).

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | success |
| `1` | the analysis ran and did not produce a result: no convergence, an infeasible dispatch, or a `convert --diff` that found differences |
| `2` | bad input or usage: an unreadable file, an unknown flag, an unrecognised extension |

`1` and `2` are deliberately distinct so a pipeline can tell "the answer is no"
from "I asked wrongly"; the split is the one proposed in
[RFC 0006](https://github.com/tpt-solutions/tpt-energy/blob/master/rfcs/0006-unify-error-handling.md).

```
