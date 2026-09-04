# Introduction

TPT Energy is a comprehensive Rust toolkit for power-systems engineering and energy-systems modeling.

The toolkit covers the full **Energy Cycle**: resource modeling (solar, wind, hydro, load), grid analysis (power flow, fault, state estimation, protection), storage (battery, hydrogen, thermal), microgrids (DERs, islanding, virtual power plants), dispatch (unit commitment, economic dispatch, reserves), and economics (LCOE, market, carbon).

The goal is to provide a unified, dependency-light, type-safe library that scales from a single 2-bus example to a 10,000-bus utility system, with optional WebAssembly bindings for in-browser interactive use.
