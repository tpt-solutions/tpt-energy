"""Smoke tests for the `tpt_nrg` extension module.

Build and run with:

    pip install maturin
    maturin develop --manifest-path crates/bindings/tpt-nrg-python/Cargo.toml
    python crates/bindings/tpt-nrg-python/tests/python_api.py
"""

import pathlib
import sys

import tpt_nrg

REPO = pathlib.Path(__file__).resolve().parents[4]
IEEE14 = REPO / "test-data" / "ieee" / "ieee14.json"

failures: list[str] = []

# A three-unit case with cost curves. The committed IEEE cases carry no cost
# curves, so economic dispatch and unit commitment need one that does.
ECONOMIC_CASE = """
{
  "id": "econ", "name": "Three unit", "base_mva": 100.0, "frequency_hz": 60.0,
  "buses": [
    {"id": 1, "name": "Slack", "type": "Slack", "voltage_magnitude_pu": 1.06},
    {"id": 2, "name": "B2", "type": "Pv", "voltage_magnitude_pu": 1.02},
    {"id": 3, "name": "B3", "type": "Pq", "load_mw": 80.0, "load_mvar": 20.0}
  ],
  "branches": [
    {"id": 1, "name": "L12", "from_bus": 1, "to_bus": 2,
     "resistance_pu": 0.01, "reactance_pu": 0.05, "rating_mva": 200.0},
    {"id": 2, "name": "L23", "from_bus": 2, "to_bus": 3,
     "resistance_pu": 0.01, "reactance_pu": 0.05, "rating_mva": 200.0}
  ],
  "generators": [
    {"id": 1, "name": "G1", "bus_id": 1, "type": "Thermal",
     "p_max_mw": 100.0, "p_min_mw": 10.0,
     "cost_curve": {"segments": [
       {"start_mw": 10.0, "end_mw": 100.0, "incremental_cost_per_mwh": 20.0}]}},
    {"id": 2, "name": "G2", "bus_id": 2, "type": "Thermal",
     "p_max_mw": 150.0, "p_min_mw": 20.0,
     "cost_curve": {"segments": [
       {"start_mw": 20.0, "end_mw": 150.0, "incremental_cost_per_mwh": 30.0}]}}
  ]
}
"""


def check(name: str, condition: bool, detail: str = "") -> None:
    if condition:
        print(f"  ok   {name}")
    else:
        failures.append(f"{name}: {detail}")
        print(f"  FAIL {name}: {detail}")


def main() -> int:
    print(f"tpt_nrg {tpt_nrg.__version__}")

    print("module surface")
    check("formats lists six formats", len(tpt_nrg.formats()) == 6, str(tpt_nrg.formats()))
    check(
        "EnergyError is exported",
        issubclass(tpt_nrg.EnergyError, Exception),
    )
    check(
        "method enum has a name",
        tpt_nrg.PowerFlowMethod.NEWTON_RAPHSON.name == "newton-raphson",
    )

    print("validation")
    check("a good document validates", tpt_nrg.validate(IEEE14.read_text(), "json") is None)
    bad = tpt_nrg.validate("not json", "json")
    check("a bad document reports a kind", bad is not None and bad.startswith("json:"), str(bad))

    print("power flow")
    system = tpt_nrg.System.from_file(str(IEEE14))
    check("bus count", system.bus_count == 14, str(system.bus_count))
    check("branch count", system.branch_count == 20, str(system.branch_count))
    result = system.power_flow()
    check("converged", result.converged, repr(result))
    check("losses are positive", result.losses_mw > 0, str(result.losses_mw))
    check("voltages are near 1 pu", all(0.9 < v < 1.1 for v in result.voltage_magnitude_pu))
    v, deg = result.bus(0)
    check("bus() returns a voltage", 0.9 < v < 1.1, str((v, deg)))
    check(
        "one loading per branch",
        len(result.branch_loading) == system.branch_count,
    )

    print("interoperability")
    matpower = system.to_matpower()
    check("matpower export", "mpc.baseMVA" in matpower, matpower[:60])
    back = tpt_nrg.System.from_text(matpower, "matpower")
    check("matpower round trip", back.bus_count == system.bus_count)
    check("psse export", "BEGIN BUS DATA" in system.to_psse())
    check("cim export", "<rdf:RDF" in system.to_cim())
    check("yaml export", "buses:" in system.to_yaml())

    print("economics and studies")
    econ = tpt_nrg.System.from_json(ECONOMIC_CASE)
    outputs, marginal, _cost = econ.economic_dispatch(100.0)
    check("dispatch covers the load", abs(sum(outputs) - 100.0) < 1.0, str(outputs))
    check("marginal cost is the cheap unit", abs(marginal - 20.0) < 1e-6, str(marginal))
    commitment, _, _ = econ.unit_commitment([100.0] * 24)
    check("commitment covers 24 h", len(commitment[0]) == 24, str(len(commitment[0])))
    check("fault current is positive", system.fault_current_pu(4) > 0)
    check("carbon intensity is positive", system.carbon_intensity() > 0)
    lcoe = system.lcoe(1.0e8, 50_000.0)
    check("lcoe is positive", lcoe.lcoe_dollar_per_mwh > 0, repr(lcoe))

    print("infeasible dispatch is reported")
    try:
        econ.economic_dispatch(10_000.0)
    except tpt_nrg.EnergyError as exc:
        check("infeasible kind", getattr(exc, "kind", None) == "infeasible", repr(exc))
    else:
        check("infeasible raised", False, "no exception")

    print("visualization")
    svg = system.to_svg()
    check("svg output", svg.startswith("<svg") and "</svg>" in svg, svg[:40])
    check("svg has one circle per bus", svg.count("<circle") >= system.bus_count)

    print("introspection")
    d = system.to_dict()
    check("to_dict is nested", isinstance(d, dict) and isinstance(d["buses"], list))
    check("to_dict has the id", d["id"] == system.id)

    print("errors carry a kind")
    try:
        tpt_nrg.System.from_json("{}")
    except tpt_nrg.EnergyError as exc:
        check("error kind attribute", getattr(exc, "kind", None) == "validation", repr(exc))
    else:
        check("error raised", False, "no exception")

    if failures:
        print(f"\n{len(failures)} failure(s)")
        return 1
    print("\nall checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
