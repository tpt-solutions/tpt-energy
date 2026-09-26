// TPT Energy playground.
//
// Everything on this page runs in the browser: the WebAssembly module is the
// same code the Rust test suite exercises, compiled by `wasm-pack`. The only
// build step is producing `playground/pkg` (see `tools/build-npm-package.sh`),
// which `playground.yml` does before deploying to GitHub Pages.

import init, {
  wasm_run_powerflow_json,
  wasm_visualize_json,
} from "./pkg/tpt_nrg_wasm.js";

const CASES = {
  ieee14: "cases/ieee14.json",
  ieee30: "cases/ieee30.json",
  ieee57: "cases/ieee57.json",
};

const elements = {
  caseSelect: document.querySelector("#case-select"),
  caseInput: document.querySelector("#case-input"),
  solve: document.querySelector("#solve"),
  status: document.querySelector("#status"),
  converged: document.querySelector("#converged"),
  iterations: document.querySelector("#iterations"),
  losses: document.querySelector("#losses"),
  voltageRange: document.querySelector("#voltage-range"),
  diagram: document.querySelector("#diagram"),
  table: document.querySelector("#voltages tbody"),
};

/// Errors crossing the WASM boundary are `{ kind, message }`; `kind` is the
/// stable part, so the UI can explain the failure instead of dumping it.
function describe(error) {
  if (error && typeof error === "object" && typeof error.kind === "string") {
    return `${error.kind}: ${error.message}`;
  }
  return String(error);
}

function setStatus(text, className = "") {
  elements.status.textContent = text;
  elements.status.className = `status ${className}`.trim();
}

function formatNumber(value, digits = 4) {
  return Number.isFinite(value) ? value.toFixed(digits) : "-";
}

/// Per-bus injections implied by the case, so the table can show what each bus
/// is asking the network for.
function busInjections(system) {
  const injections = new Map();
  const add = (busId, pMw, qMvar) => {
    const current = injections.get(busId) ?? { p: 0, q: 0 };
    injections.set(busId, { p: current.p + pMw, q: current.q + qMvar });
  };
  for (const bus of system.buses ?? []) {
    add(
      bus.id,
      (bus.generation_mw ?? 0) - (bus.load_mw ?? 0),
      (bus.generation_mvar ?? 0) - (bus.load_mvar ?? 0),
    );
  }
  for (const generator of system.generators ?? []) {
    add(generator.bus_id, generator.p_schedule_mw ?? 0, 0);
  }
  for (const load of system.loads ?? []) {
    add(load.bus_id, -(load.p_mw ?? 0), -(load.q_mvar ?? 0));
  }
  return injections;
}

function renderTable(system, result) {
  const injections = busInjections(system);
  const rows = (system.buses ?? []).map((bus, index) => {
    const injection = injections.get(bus.id) ?? { p: 0, q: 0 };
    return `<tr>
      <td>${bus.id}</td>
      <td>${formatNumber(result.voltage_magnitude_pu?.[index])}</td>
      <td>${formatNumber(result.voltage_angle_rad?.[index])}</td>
      <td>${formatNumber(injection.p, 2)}</td>
      <td>${formatNumber(injection.q, 2)}</td>
    </tr>`;
  });
  elements.table.innerHTML = rows.join("");
}

function renderSummary(system, result) {
  const voltages = result.voltage_magnitude_pu ?? [];
  const lowest = voltages.length ? Math.min(...voltages) : NaN;
  const highest = voltages.length ? Math.max(...voltages) : NaN;
  elements.converged.textContent = result.converged ? "yes" : "no";
  elements.iterations.textContent = String(result.iterations ?? "-");
  elements.losses.textContent = `${formatNumber(result.losses_mw, 3)} MW`;
  elements.voltageRange.textContent = voltages.length
    ? `${formatNumber(lowest)} - ${formatNumber(highest)} pu`
    : "-";
  elements.converged.style.color = result.converged ? "var(--ok)" : "var(--bad)";
  document.title = `${system.name ?? "case"} - TPT Energy playground`;

async function readCase() {
  const selected = elements.caseSelect.value;
  if (selected === "custom") {
    const text = elements.caseInput.value.trim();
    if (!text) {
      throw { kind: "json", message: "paste a case, or pick a bundled one" };
    }
    return text;
  }
  const response = await fetch(CASES[selected]);
  if (!response.ok) {
    throw { kind: "json", message: `cannot load ${CASES[selected]} (${response.status})` };
  }
  return response.text();
}

async function solve() {
  elements.solve.disabled = true;
  setStatus("loading the WebAssembly module...");
  try {
    await init();
    setStatus("solving...");
    const text = await readCase();
    const result = wasm_run_powerflow_json(text);
    const system = JSON.parse(text);
    renderSummary(system, result);
    renderTable(system, result);
    // The visualiser re-solves internally and returns a self-contained SVG
    // string produced by `tpt-nrg-viz`, so it can be injected as-is.
    const svg = wasm_visualize_json(JSON.stringify({ system: text }));
    elements.diagram.innerHTML = svg;
    setStatus(
      result.converged ? `converged in ${result.iterations} iterations` : "did not converge",
      result.converged ? "ok" : "error",
    );
  } catch (error) {
    setStatus(describe(error), "error");
  } finally {
    elements.solve.disabled = false;
  }
}

elements.solve.addEventListener("click", solve);
elements.caseSelect.addEventListener("change", () => {
  const custom = elements.caseSelect.value === "custom";
  elements.caseInput.closest("details").open = custom;
  if (custom) {
    setStatus("paste a case, then press Solve");
  } else {
    solve();
  }
});

// Solve the default case on load so the page is never blank.
solve();

}
