# RFC 0001 — Sparse Power-Flow Solver

| Field         | Value                                     |
|---------------|-------------------------------------------|
| Status        | Proposed                                  |
| Author        | TPT Energy maintainers                    |
| Created       | 2026-09-04                                |
| Tracking      | https://github.com/tpt-solutions/tpt-energy/issues |

## Summary

Replace the dense Y-bus representation in `tpt-nrg-topology` and
`tpt-nrg-powerflow` with a sparse format (`csr`/`csc`) and integrate a
sparse LU solver. This is the gating item for solving the IEEE
118/300-bus systems (Phase 6 milestone) within reasonable time and
memory.

## Motivation

The current dense representation allocates `O(n²)` doubles per Y-bus
and performs Gaussian elimination with `O(n³)` flops. For 1000+ bus
systems this becomes prohibitive. Sparse storage reduces memory by a
factor of 10-50 and direct sparse solvers (KLU, SuiteSparse) cut solve
time by a similar factor.

## Design

- Add `tpt-math-linalg-fixed::SparseMatrix` (CSC) as the substrate
  representation.
- Provide a `From<&EnergySystem>` adapter in `tpt-nrg-topology`.
- Add `solve_sparse` to the Newton–Raphson solver.
- Keep the dense path as the default until the substrate crate is
  available; feature-gate behind `tpt-substrate = ["sparse"]`.

## Drawbacks

- Two solver paths to maintain.
- Need careful ordering and refactorization for numerical stability.

## Alternatives

- Use the existing dense solver and accept the O(n³) cost.
- Adopt `nalgebra-sparse` directly without the substrate wrapper.

## Open Questions

- Should we expose sparse factorisation as a generic API?
- Which ordering (AMD, COLAMD) for the IEEE test systems?
