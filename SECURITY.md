# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

The TPT Energy maintainers take security seriously. **Please do not file a
public issue** for suspected vulnerabilities.

Instead, please report security issues privately via GitHub's
[private vulnerability reporting](https://github.com/tpt-solutions/tpt-energy/security/advisories/new)
or by emailing **[email protected]**.

We will:

1. Acknowledge receipt within **3 business days**.
2. Provide an initial assessment within **10 business days**.
3. Coordinate a disclosure timeline with you, with the goal of releasing a
   patch before public disclosure.

## Scope

The TPT Energy crates are computational; they do not handle authentication,
network input, or untrusted user data beyond the JSON deserialization surface.
The primary security concerns are:

- **Resource exhaustion** from large or pathological input data (mitigated by
  input validation in `tpt-nrg-core::EnergySystem`).
- **Numerical correctness** for safety-critical simulations (mitigated by
  golden tests, convergence checks, and CI fuzzing).
- **Supply chain** (mitigated by `cargo deny` and pinned dependencies).
