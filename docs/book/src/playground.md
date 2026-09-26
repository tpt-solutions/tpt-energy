# Browser playground

`playground/` is a static page that runs the WebAssembly build of TPT Energy in
the browser: pick an IEEE test case or paste your own, solve it, and read the
voltage profile and the single-line diagram. There is no server, no framework,
and no build step beyond producing the `.wasm` module.

This page covers what it is and how to run it. For the library behind it, see
[Building for the browser](wasm-build.md).

## What it shows

- Whether the solve converged, in how many iterations, and the active losses.
- The lowest and highest per-bus voltage magnitude.
- An SVG single-line diagram coloured by voltage and branch loading, rendered
  by `tpt-nrg-viz` inside the same WebAssembly module.
- A per-bus table of voltage magnitude, angle, and the injection the case asks
  for.

Failures are the other half of the interface: the WASM boundary returns
`{ kind, message }`, and the page switches on `kind` (`json`, `validation`,
`non_convergence`, `infeasible`, `control`, `serialization`) so a rejected case
says what is wrong with it instead of printing a stack.

## Run it locally

```sh
# 1. Build the WebAssembly package.
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
tools/build-npm-package.sh

# 2. Assemble the page: the package plus the committed test cases.
mkdir -p playground/pkg playground/cases
cp dist/npm/tpt_nrg_wasm* playground/pkg/
cp test-data/ieee/ieee14.json test-data/ieee/ieee30.json \
   test-data/ieee/ieee57.json playground/cases/

# 3. Serve it. A file:// URL will not work: browsers refuse to fetch the WASM
#    module and the case files over file://.
python -m http.server --directory playground
```

Then open <http://localhost:8000>.

## Check it without a browser

`playground/tests/smoke.mjs` imports the same module the page does and asserts
the contracts the page relies on: the IEEE 14-bus case converges with a
plausible voltage profile, the visualiser returns SVG, and a malformed case
throws a *typed* error. It runs in Node, so CI needs no browser.

```sh
node playground/tests/smoke.mjs
```

## Deploy it

`.github/workflows/playground.yml` builds the package, assembles the directory,
runs the smoke test, and publishes the result to GitHub Pages on every push to
`master`. Nothing about the page depends on the deployment: it is a directory
of files.

## Privacy

The case never leaves the browser. There is no analytics, no fetch to anywhere
but the page's own directory, and the solver is the same Rust code the test
suite exercises — the browser build is not a simplified reimplementation.
