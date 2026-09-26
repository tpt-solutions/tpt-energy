# Worked Example: Contingency Analysis

The question this example answers is the one every planner starts with: *"I
have a case from a colleague in MATLAB format. Can I trust it, and what happens
when a line trips?"*

Run it with:

```
cargo run --manifest-path examples/Cargo.toml --bin contingency-analysis
```

## What it does

The runnable source is `examples/contingency-analysis.rs`. It walks four steps.

**1. Import, and check the import.** It reads the committed IEEE 14-bus JSON
case, writes it out as MATPOWER, reads it back, and compares the two. This check
is the point, not a formality: a silent id or bus-type mismatch during import
would invalidate every downstream number, so the example asserts on bus ids,
bus types, branch endpoints, and record counts before trusting the result.

The MATPOWER text is generated in-process so the example is self-contained. To
run it against a real `case*.m`, point `repo_path("case14.m")` at the file.

**2. Solve the base case.** Newton-Raphson, reporting the converged
iteration count, total losses, lowest bus voltage, and most loaded branch.

**3. Enumerate contingencies.** Every in-service branch is taken out of service
in turn and the case re-solved. Two details matter:

- A contingency that will not converge is a *result*, not a crash. The whole
  point of the sweep is to find the cases that do not hold up, so the example
  reports divergence rather than propagating an error.
- An outage that islands the network is skipped and said so. A floating island
  has no slack bus, so the base solver is not the right tool for it, and
  reporting a number would be worse than reporting nothing. The example
  detects this by checking that every connected component has exactly one
  slack.

**4. Summarise.** How many cases diverged, how many violated the voltage
limits, how many overloaded a branch, and which are worst. The severity
ranking weights a voltage sag more heavily than an overload, because a sagging
bus usually affects more customers than a single loaded branch.

It finishes with a three-phase short-circuit level at the weakest bus, since
that is the natural companion number to an outage study.

## Reading the output

On the committed IEEE 14-bus case, all 19 solvable contingencies converge and
none violates the 0.95 pu voltage floor, but four overload a branch — the
1-2 outage is the worst at 175% of rating. The 1-2 outage is also the one that
islands 7-8, so the example prints that it is skipping it rather than
inventing a result for the island.

IEEE 14-bus is a small teaching case and is not operated as a real system, so
these overloads are a property of the test data, not a finding about any
utility. The example demonstrates the method; it does not certify a network.
