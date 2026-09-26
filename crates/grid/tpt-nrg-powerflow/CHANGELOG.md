# Changelog
All notable changes to this crate will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Newton–Raphson AC solver with damped steps, DC warm start, and PV-to-PQ switching at generator reactive-power limits (Dommel–Tinney outer loop).
- Gauss–Seidel, Fast Decoupled (Stott–Alsac XB), and DC linear solvers behind one `PowerFlowSolver` API.
- `PowerFlowResult` with voltages, angles, branch flows/loadings, losses, and per-generator P/Q dispatch.
- Golden fixtures for IEEE 14-bus, 30-bus, and 57-bus under `test-data/golden/powerflow/`, with published-MATPOWER anchor assertions for 57-bus.
- This crate-level `README.md` with crates.io `categories` and `keywords` metadata.

### Changed
- `PowerFlowSolver::solve` documents its error contract via a `# Errors` section.
- DC branch-flow and B-matrix construction extracted into testable helpers.

### Fixed
- IEEE 57-bus now converges under full AC Newton–Raphson in 5 iterations and reproduces the published MATPOWER case57 solution (losses ≈ 27.86 MW, slack ≈ 478.66 MW / 128.85 MVAr) within 1% — previously the solver plateaued tens of MW away.
- The convergence check takes the absolute mismatch (a negative-mismatch regression previously registered as zero progress).
- Q-limit switching fires only on near-converged iterates, preventing transient over-constraining, and a limit change can no longer be followed by an immediate false convergence declaration.
- Generator reactive limits are checked against the generator's output (net injection plus bus load), not the net injection alone.
- The DC B-matrix diagonal includes the removed slack column (angles in multi-bus systems with shunted branches were previously corrupted).

## License

Dual-licensed under MIT or Apache-2.0. See the repository root
`LICENSE-MIT` / `LICENSE-APACHE` files.
