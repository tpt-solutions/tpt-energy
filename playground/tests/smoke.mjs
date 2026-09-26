// Smoke test for the assembled playground.
//
// The page itself is the thing that matters, but a page that throws on load
// only fails in a browser, which CI does not have. This script exercises the
// same WebAssembly module the page imports -- power flow, validation,
// visualisation, and a typed failure -- so a renamed export or a broken
// request shape is caught here instead of by a reader.
//
// Usage: node playground/tests/smoke.mjs   (after `tools/build-npm-package.sh`
// and copying `dist/npm/tpt_nrg_wasm*` into `playground/pkg/`)

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

// `initSync` is a *named* export; the default export is the async `init` the
// browser page uses. Importing the default under the name `initSync` would
// call the async initialiser without awaiting it, and the first export call
// would fail with an undefined instance.
import init, {
  initSync,
  wasm_carbon_intensity_json,
  wasm_economic_dispatch_json,
  wasm_run_powerflow_json,
  wasm_validate_system_json,
  wasm_visualize_json,
} from "../pkg/tpt_nrg_wasm.js";

const playground = new URL("../", import.meta.url);
const ieee14 = await readFile(new URL("cases/ieee14.json", playground), "utf8");

// The web build ships an ES module that fetches its `.wasm`; in Node the
// bytes are handed over directly.
const wasmBytes = await readFile(new URL("pkg/tpt_nrg_wasm_bg.wasm", playground));
initSync({ module: wasmBytes });

// --- the power flow the page runs on load -----------------------------------
const result = wasm_run_powerflow_json(ieee14);
assert.equal(result.converged, true, "the IEEE 14-bus case must converge");
assert.ok(result.iterations > 0, "the solver must report its iterations");
assert.equal(result.voltage_magnitude_pu.length, 14, "one voltage per bus");
assert.equal(result.voltage_angle_rad.length, 14, "one angle per bus");
assert.ok(result.losses_mw > 0, "a lossy network reports positive losses");
assert.ok(
  result.voltage_magnitude_pu.every((v) => v > 0.8 && v < 1.2),
  "voltages stay within a planning band",
);

// --- the diagram the page injects -------------------------------------------
const svg = wasm_visualize_json(JSON.stringify({ system: ieee14 }));
assert.ok(svg.startsWith("<svg"), "the visualiser returns SVG");
assert.ok(svg.includes("</svg>"), "the SVG is closed");

// --- validation -------------------------------------------------------------
wasm_validate_system_json(ieee14);

// --- the other exports the page does not use yet ---------------------------
// The bundled IEEE cases carry no cost curves, so an economic dispatch on them
// is infeasible by construction. That is the honest answer, and the typed
// error is the contract a UI depends on, so assert that first.
let dispatchError;
try {
  wasm_economic_dispatch_json(JSON.stringify({ system: ieee14, load_mw: 100 }));
} catch (error) {
  dispatchError = error;
}
assert.ok(dispatchError, "a case with no merit order cannot be dispatched");
assert.equal(dispatchError.kind, "infeasible");

// Give the cheapest unit a merit order and the same call has to work.
const dispatchable = JSON.parse(ieee14);
dispatchable.generators[0].cost_curve = {
  no_load_cost: 0,
  startup_cost: 0,
  segments: [{ start_mw: 0, end_mw: 332.4, incremental_cost_per_mwh: 22 }],
};
// The four "study" exports mirror the Rust API, which is JSON-in/JSON-out, so
// they hand back a JSON *string* rather than a typed object. Only the power
// flow and the visualiser are structured across the boundary.
const dispatch = JSON.parse(
  wasm_economic_dispatch_json(
    JSON.stringify({ system: JSON.stringify(dispatchable), load_mw: 100 }),
  ),
);
assert.equal(
  dispatch.generator_outputs_mw.length,
  dispatchable.generators.length,
);
assert.ok(dispatch.generator_outputs_mw[0] > 90, "the cheapest unit carries the load");
assert.ok(dispatch.marginal_cost_dollar_per_mwh > 0, "a cost is reported");

const carbon = JSON.parse(wasm_carbon_intensity_json(ieee14));
assert.equal(typeof carbon.carbon_intensity_kg_per_mwh, "number");

// --- typed failures ---------------------------------------------------------
// A JS caller switches on `kind`; that contract is what makes the page able to
// explain a failure instead of printing `undefined`.
let thrown;
try {
  wasm_validate_system_json("{ not json");
} catch (error) {
  thrown = error;
}
assert.ok(thrown, "malformed JSON must throw");
assert.ok(
  ["json", "validation"].includes(thrown.kind),
  `expected a typed failure, got ${JSON.stringify(thrown)}`,
);
assert.equal(typeof thrown.message, "string");

console.log("playground smoke test passed");
