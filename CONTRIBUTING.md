# Contributing to tpt-energy

Thank you for your interest in contributing to TPT Energy! This document
describes the workflow and standards for contributions.

## Workflow

1. **Fork** the repository on GitHub.
2. **Create a branch** for your work:
   - `feature/<short-description>` for new features
   - `fix/<short-description>` for bug fixes
   - `docs/<short-description>` for documentation only
   - `rfc/<short-description>` for new RFC proposals
3. **Write code + tests.** All new functionality must come with unit tests
   and, where applicable, golden tests against published reference data.
4. **Format, lint, and test** locally:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all
   cargo deny check licenses
   ```
5. **Sign off your commits** with the [DCO](https://developercertificate.org/):
   ```bash
   git commit -s -m "feat: add Jensen wake model for offshore wind"
   ```
6. **Open a Pull Request** using the provided PR template.
7. **RFCs.** If your change is substantial (new crate, new algorithm, breaking
   API change), file an RFC under `rfcs/` first and obtain maintainer approval.
8. **Approvals.** PRs require **2 approvals** from maintainers before merge.

## Coding Standards

- `cargo fmt` — Rustfmt (see `rustfmt.toml`)
- `cargo clippy --all-targets -- -D warnings` — no warnings
- `cargo doc --no-deps` — clean docs
- Public APIs must be documented with `///` doc comments
- All public items must have unit tests
- Performance-critical code must have benchmarks in `benches/`

## License

By contributing, you agree that your contributions will be dual-licensed under
MIT OR Apache-2.0, matching the project license.
