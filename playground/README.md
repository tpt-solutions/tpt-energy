# TPT Energy playground

A static page that solves an electrical power system in the browser: pick an
IEEE test case (or paste your own), run a Newton-Raphson power flow through the
WebAssembly build of [TPT Energy](https://github.com/tpt-solutions/tpt-energy),
and read the voltage profile and the single-line diagram.

There is no server and no framework: `index.html`, `styles.css`, `app.js`, and
the generated `pkg/` directory are the whole thing.

## Run it locally

```sh
# 1. Build the WebAssembly package (needs the wasm32 target and wasm-pack).
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
tools/build-npm-package.sh

# 2. Assemble the page: the package plus the committed test cases.
mkdir -p playground/pkg playground/cases
cp dist/npm/tpt_nrg_wasm* playground/pkg/
cp test-data/ieee/ieee14.json test-data/ieee/ieee30.json \
   test-data/ieee/ieee57.json playground/cases/

# 3. Serve it. A file:// URL will not work: the WASM module and the cases are
#    fetched, and browsers refuse both over file://.
python -m http.server --directory playground
```

Then open <http://localhost:8000>.

## Deploy it

`.github/workflows/playground.yml` builds the package, assembles the same
directory, and publishes it to GitHub Pages on every push to `master`. The
same workflow runs `playground/tests/smoke.mjs` against the assembled page, so
a broken import or a renamed export fails the build instead of the page.

## What the page shows

- Whether the solve converged, in how many iterations, and the active losses.
- The lowest and highest per-bus voltage magnitude.
- An SVG single-line diagram coloured by voltage and branch loading, rendered
  by `tpt-nrg-viz` inside the same WebAssembly module.
- A per-bus table of voltage magnitude, angle, and the injection the case
  asks for.

## Licence

Dual licensed under MIT or Apache-2.0, matching the TPT Energy workspace.
