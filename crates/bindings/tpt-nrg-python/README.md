# tpt-nrg (Python)

Python bindings for [TPT Energy](https://github.com/tpt-solutions/tpt-energy):
power flow, economic dispatch, unit commitment, short-circuit analysis, LCOE,
carbon intensity, case-format interoperability, and SVG rendering.

Part of [TPT Energy](https://github.com/tpt-solutions/tpt-energy) — power
systems, resource modeling, storage, dispatch, and grid economics in pure Rust.

## Install

```sh
pip install tpt-nrg
```

Building from a checkout needs a stable Rust toolchain and Python 3.9+:

```sh
maturin develop --manifest-path crates/bindings/tpt-nrg-python/Cargo.toml
```

## Usage

```python
import tpt_nrg

# Load a case. The format is inferred from the file extension:
# .json .yaml .csv .m (MATPOWER) .raw (PSS/E) .xml (CIM)
system = tpt_nrg.System.from_file("test-data/ieee/ieee14.json")

result = system.power_flow(tpt_nrg.PowerFlowMethod.NEWTON_RAPHSON)
print(result.converged, result.iterations, result.losses_mw)

for i, v in enumerate(result.voltage_magnitude_pu):
    print(f"bus {i}: {v:.4f} pu")
```

### Dispatch and economics

```python
outputs, marginal, cost = system.economic_dispatch(load_mw=100.0)
commitment, schedule, total = system.unit_commitment([100.0] * 24)
print(system.carbon_intensity(), "kg CO2/MWh")
print(system.lcoe(capex_dollar=1e8, annual_energy_mwh=50_000).lcoe_dollar_per_mwh)
```

### Short circuit

```python
print(system.fault_current_pu(bus=4), "pu")
```

### Interoperability

Every format is two-way, and the format is a plain string:

```python
tpt_nrg.formats()
# ['json', 'yaml', 'csv', 'matpower', 'psse', 'cim']

matpower = system.to_matpower()
psse = system.to_psse()
cim = system.to_cim()
back = tpt_nrg.System.from_text(matpower, "matpower")
```

### Visualization

```python
svg = system.to_svg()                       # single-line diagram + heatmap
open("diagram.svg", "w").write(svg)
```

### Validation

`validate` returns `None` for a good document and a `"kind: message"` string
otherwise, so a caller can check a case without paying for a solve:

```python
print(tpt_nrg.validate(open("case.json").read(), "json"))
```

## Errors

Every failure is a `tpt_nrg.EnergyError` (a subclass of `ValueError`) carrying
a stable `kind` attribute, so Python code branches on the failure rather than
on message text:

| `kind`          | Meaning                                            |
|-----------------|----------------------------------------------------|
| `validation`    | parsed, but not a structurally valid system         |
| `non_convergence` | the solver did not converge                      |
| `infeasible`    | the dispatch problem has no solution               |
| `unsupported`   | unknown format or extension                        |
| `parse`         | the document could not be serialized                |
| `io`            | a file could not be read                           |

```python
try:
    system.power_flow(max_iterations=1)
except tpt_nrg.EnergyError as exc:
    if exc.kind == "non_convergence":
        ...  # loosen the tolerance or fix the case
```

## Status

**Alpha.** The API is stable, but it mirrors a 0.1.x Rust crate and may gain
methods. `tpt_nrg.__version__` reports the crate version.

## Testing

```sh
maturin develop --manifest-path crates/bindings/tpt-nrg-python/Cargo.toml
python crates/bindings/tpt-nrg-python/tests/python_api.py
```

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-energy/blob/master/LICENSE-APACHE).
