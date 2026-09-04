#!/usr/bin/env python3
"""Convert MATPOWER case_ieee57 / case_ieee118 .m sources to tpt-energy JSON.

Usage:
    python tools/matpower_to_json.py
"""

import json
import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
TEST_DATA_IEEE = REPO / "test-data" / "ieee"
GOLDEN_DIR = REPO / "test-data" / "golden" / "powerflow"


def parse_matpower_matrix(text: str, name: str) -> list[list[float]]:
    """Extract a numeric matrix from a MATPOWER .m file by block name."""
    m = re.search(rf"mpc\.{name}\s*=\s*\[(.*?)\];", text, re.DOTALL)
    if not m:
        raise ValueError(f"block mpc.{name} not found")
    rows: list[list[float]] = []
    for line in m.group(1).splitlines():
        line = line.split("%", 1)[0].strip()
        if not line.endswith(";"):
            continue
        line = line.rstrip(";").strip()
        if not line:
            continue
        rows.append([float(x) for x in line.split()])
    return rows


def case57_to_json() -> dict:
    src = REPO / "tools" / "_matpower_case57.m"
    text = src.read_text()
    bus = parse_matpower_matrix(text, "bus")
    gen = parse_matpower_matrix(text, "gen")
    branch = parse_matpower_matrix(text, "branch")
    base_mva = 100.0

    # MATPOWER bus columns: 0:bus_i 1:type 2:Pd 3:Qd 4:Gs 5:Bs 6:area 7:Vm
    # 8:Va(deg) 9:baseKV 10:zone 11:Vmax 12:Vmin
    # type 3 = slack (REF), 2 = PV (GEN), 1 = PQ, 4 = isolated
    bus_type_map = {1: "Pq", 2: "Pv", 3: "Slack", 4: "Isolated"}

    buses = []
    for row in bus:
        bus_id = int(row[0])
        btype = bus_type_map[int(row[1])]
        vm = row[7]
        va_deg = row[8]
        va_rad = va_deg * 3.141592653589793 / 180.0
        base_kv = row[9] if row[9] > 0 else 100.0
        d = {
            "id": bus_id,
            "name": f"Bus {bus_id}",
            "type": btype,
            "voltage_magnitude_pu": round(vm, 4),
            "voltage_angle_rad": round(va_rad, 4),
            "base_kv": round(base_kv, 1),
        }
        if row[2] != 0:
            d["load_mw"] = round(row[2], 4)
        if row[3] != 0:
            d["load_mvar"] = round(row[3], 4)
        if row[4] != 0:
            d["shunt_conductance_pu"] = row[4]
        if row[5] != 0:
            d["shunt_susceptance_pu"] = row[5]
        buses.append(d)

    # branch cols: 0:fbus 1:tbus 2:r 3:x 4:b 5..7:rateA/B/C 8:ratio 9:angle
    # 10:status
    branches = []
    for i, row in enumerate(branch, start=1):
        d = {
            "id": i,
            "name": f"{int(row[0])}-{int(row[1])}",
            "from_bus": int(row[0]),
            "to_bus": int(row[1]),
            "resistance_pu": row[2],
            "reactance_pu": row[3],
            "susceptance_pu": row[4],
            "rating_mva": 100.0,
        }
        if row[8] != 0:
            d["tap_ratio"] = row[8]
        if row[9] != 0:
            d["phase_shift_rad"] = row[9] * 3.141592653589793 / 180.0
        branches.append(d)

    # gen cols: 0:bus 1:Pg 2:Qg 3:Qmax 4:Qmin 5:Vg 6:mBase 7:status 8:Pmax 9:Pmin
    generators = []
    for i, row in enumerate(gen, start=1):
        d = {
            "id": i,
            "name": f"G{int(row[0])}",
            "bus_id": int(row[0]),
            "type": "Thermal",
            "p_max_mw": row[8],
            "p_min_mw": row[9],
            "p_schedule_mw": row[1],
            "voltage_setpoint_pu": row[5],
            "q_max_mvar": row[3],
            "q_min_mvar": row[4],
        }
        generators.append(d)

    # NOTE: do NOT copy generation into b.generation_mw. The `Generator`
    # objects below already feed `bus_schedules_pu` (the powerflow schedules
    # net injection from `g.p_schedule_mw - load_mw`). Setting bus.generation_mw
    # would double-count.
    return {
        "id": "ieee57",
        "name": "IEEE 57-Bus Test Case",
        "base_mva": base_mva,
        "frequency_hz": 60.0,
        "buses": buses,
        "branches": branches,
        "generators": generators,
        "metadata": {"source": "MATPOWER case57.m", "base_mva": base_mva},
    }


def case118_to_json() -> dict:
    src = REPO / "tools" / "_matpower_case118.m"
    text = src.read_text()
    bus = parse_matpower_matrix(text, "bus")
    gen = parse_matpower_matrix(text, "gen")
    branch = parse_matpower_matrix(text, "branch")
    base_mva = 100.0

    bus_type_map = {1: "Pq", 2: "Pv", 3: "Slack", 4: "Isolated"}

    buses = []
    for row in bus:
        bus_id = int(row[0])
        btype = bus_type_map[int(row[1])]
        vm = row[7]
        va_deg = row[8]
        va_rad = va_deg * 3.141592653589793 / 180.0
        base_kv = row[9] if row[9] > 0 else 100.0
        d = {
            "id": bus_id,
            "name": f"Bus {bus_id}",
            "type": btype,
            "voltage_magnitude_pu": round(vm, 4),
            "voltage_angle_rad": round(va_rad, 4),
            "base_kv": round(base_kv, 1),
        }
        if row[2] != 0:
            d["load_mw"] = round(row[2], 4)
        if row[3] != 0:
            d["load_mvar"] = round(row[3], 4)
        if row[4] != 0:
            d["shunt_conductance_pu"] = row[4]
        if row[5] != 0:
            d["shunt_susceptance_pu"] = row[5]
        buses.append(d)

    branches = []
    for i, row in enumerate(branch, start=1):
        d = {
            "id": i,
            "name": f"{int(row[0])}-{int(row[1])}",
            "from_bus": int(row[0]),
            "to_bus": int(row[1]),
            "resistance_pu": row[2],
            "reactance_pu": row[3],
            "susceptance_pu": row[4],
            "rating_mva": row[5] if row[5] > 0 else 100.0,
        }
        if row[8] != 0:
            d["tap_ratio"] = row[8]
        if row[9] != 0:
            d["phase_shift_rad"] = row[9] * 3.141592653589793 / 180.0
        branches.append(d)

    generators = []
    for i, row in enumerate(gen, start=1):
        d = {
            "id": i,
            "name": f"G{int(row[0])}",
            "bus_id": int(row[0]),
            "type": "Thermal",
            "p_max_mw": row[8],
            "p_min_mw": row[9],
            "p_schedule_mw": row[1],
            "voltage_setpoint_pu": row[5],
            "q_max_mvar": row[3],
            "q_min_mvar": row[4],
        }
        generators.append(d)

    for g in generators:
        for b in buses:
            if b["id"] == g["bus_id"]:
                b["generation_mw"] = g["p_schedule_mw"]

    return {
        "id": "ieee118",
        "name": "IEEE 118-Bus Test Case",
        "base_mva": base_mva,
        "frequency_hz": 60.0,
        "buses": buses,
        "branches": branches,
        "generators": generators,
        "metadata": {"source": "MATPOWER case118.m", "base_mva": base_mva},
    }


def main() -> None:
    TEST_DATA_IEEE.mkdir(parents=True, exist_ok=True)
    GOLDEN_DIR.mkdir(parents=True, exist_ok=True)
    if (REPO / "tools" / "_matpower_case57.m").exists():
        out = TEST_DATA_IEEE / "ieee57.json"
        out.write_text(json.dumps(case57_to_json(), indent=2))
        print(f"wrote {out}")
    if (REPO / "tools" / "_matpower_case118.m").exists():
        out = TEST_DATA_IEEE / "ieee118.json"
        out.write_text(json.dumps(case118_to_json(), indent=2))
        print(f"wrote {out}")


if __name__ == "__main__":
    main()
