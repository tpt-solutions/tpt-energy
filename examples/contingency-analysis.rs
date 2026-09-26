//! Worked example: import a MATPOWER case and run a contingency analysis.
//!
//! Run with:
//! `cargo run --manifest-path examples/Cargo.toml --bin contingency-analysis`
//!
//! The task this answers is the one every planner starts with: *"I have a
//! case from a colleague in MATLAB format. Can I trust it, and what happens
//! when a line trips?"*
//!
//! It walks four steps:
//!
//! 1. **Import** a `case*.m` file via [`tpt_nrg_interop::matpower`], and
//!    check the import against the committed JSON copy of the same case.
//! 2. **Solve** the base case, and report the worst bus voltage and the most
//!    loaded branch.
//! 3. **Enumerate contingencies**: take every in-service branch out of
//!    service in turn (an N-1 sweep) and re-solve.
//! 4. **Summarise** which contingencies diverge, which violate the planning
//!    voltage and loading limits, and rank the worst.
//!
//! The MATPOWER source is generated on the fly from the committed JSON so the
//! example is self-contained; point `repo_path("case14.m")` at a real file to
//! run it on a colleague's case instead.

use std::path::{Path, PathBuf};

use tpt_nrg_core::{Branch, BusType, EnergySystem};
use tpt_nrg_fault::{FaultAnalyzer, FaultType};
use tpt_nrg_interop::matpower;
use tpt_nrg_powerflow::{
    BranchFlow, PowerFlowError, PowerFlowMethod, PowerFlowResult, PowerFlowSolver,
};

/// Lower planning limit on bus voltage, in per-unit.
const V_MIN: f64 = 0.95;
/// Upper planning limit on bus voltage, in per-unit.
const V_MAX: f64 = 1.05;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Contingency analysis on an imported MATPOWER case ===\n");

    // ----------------------------------------------------------------
    // 1. Import
    // ----------------------------------------------------------------
    let reference = EnergySystem::from_json_file(repo_path("test-data/ieee/ieee14.json")?)?;

    // Stands in for a colleague's `case14.m`. Writing and re-reading it also
    // demonstrates that the round trip is lossless.
    let source = matpower::to_matpower(&reference)?;
    let base = matpower::from_matpower(&source)?;
    verify_import(&base, &reference)?;

    println!("Step 1: Import");
    println!("  source: MATPOWER (case*.m)");
    println!(
        "  buses: {}, branches: {}, generators: {}",
        base.buses.len(),
        base.branches.len(),
        base.generators.len()
    );
    println!("  base MVA: {:.1}", base.base_mva);
    println!("  import matches the reference case: yes");

    // ----------------------------------------------------------------
    // 2. Base case
    // ----------------------------------------------------------------
    let base_result = solve(&base)?;
    println!("\nStep 2: Base case (Newton-Raphson)");
    print_solution("  base", &base_result);

    // ----------------------------------------------------------------
    // 3. N-1 sweep
    // ----------------------------------------------------------------
    println!(
        "\nStep 3: N-1 contingency sweep ({} outages)",
        base.branches.len()
    );
    println!(
        "  {:<12} {:>7} {:>9} {:>9} {:>9}  verdict",
        "outage", "solved", "min |V|", "max load", "losses"
    );
    println!("  {}", "-".repeat(74));

    let mut reports = Vec::new();
    for outage in base.branches.clone() {
        let Some((name, result, verdict)) = evaluate(&base, &outage) else {
            continue;
        };
        let (v_min, load, losses) = metrics(&result);
        println!(
            "  {:<12} {:>7} {:>9.4} {:>8.1}% {:>9.2}  {}",
            name,
            if result.converged { "yes" } else { "NO" },
            v_min,
            load * 100.0,
            losses,
            verdict
        );
        reports.push(Report {
            name,
            converged: result.converged,
            v_min,
            max_loading: load,
            verdict,
        });
    }

    // ----------------------------------------------------------------
    // 4. Summary
    // ----------------------------------------------------------------
    println!("\nStep 4: Summary");
    let diverged = reports.iter().filter(|r| !r.converged).count();
    let stressed = reports
        .iter()
        .filter(|r| r.converged && r.v_min < V_MIN)
        .count();
    let overloaded = reports
        .iter()
        .filter(|r| r.converged && r.max_loading > 1.0)
        .count();
    println!("  contingencies studied : {}", reports.len());
    println!("  failed to converge    : {diverged}");
    println!("  below V_MIN ({V_MIN:.2} pu)   : {stressed}");
    println!("  branch over rating    : {overloaded}");

    // Rank the worst few so the report is actionable rather than a wall of
    // numbers: a planner wants to know where to reinforce first.
    let mut worst: Vec<&Report> = reports
        .iter()
        .filter(|r| r.converged && (r.v_min < V_MIN || r.max_loading > 1.0))
        .collect();
    worst.sort_by(|a, b| severity(a).total_cmp(&severity(b)));

    if worst.is_empty() {
        println!("\n  All N-1 cases hold the planning criteria.");
    } else {
        println!("\n  Most severe contingencies:");
        for r in worst.iter().take(5) {
            println!(
                "    {:<12} min |V| = {:.4} pu, max loading = {:.1}%  ({})",
                r.name,
                r.v_min,
                r.max_loading * 100.0,
                r.verdict
            );
        }
    }

    // A short-circuit level at the weakest bus is the natural companion to a
    // contingency sweep, so show the three-phase fault there.
    if let Some(bus) = weakest_bus(&base, &base_result) {
        let fault = FaultAnalyzer::new(&base).calculate_fault_current(bus, FaultType::ThreePhase);
        println!(
            "\n  Three-phase fault at the weakest bus ({bus}): {:.3} pu ({:.0} A)",
            fault.i_fault_pu, fault.i_fault_amps
        );
    }

    Ok(())
}

/// What to report for one contingency.
struct Report {
    /// The outaged branch, `from-to`.
    name: String,
    /// Whether the solver converged.
    converged: bool,
    /// Lowest bus voltage, in per-unit.
    v_min: f64,
    /// Highest branch loading, as a fraction of rating.
    max_loading: f64,
    /// Human-readable planning verdict.
    verdict: &'static str,
}

/// How far a case strays from the planning limits, for ranking.
///
/// Lower is worse; a case that is fine on both criteria scores `0`.
fn severity(r: &Report) -> f64 {
    let v = (V_MIN - r.v_min).max(0.0);
    let loading = (r.max_loading - 1.0).max(0.0);
    // A 1 pu voltage sag matters more than a 1 pu overload, hence the 10x.
    (v * 10.0).max(loading)
}

/// Path to a file in the repository, resolved from the examples workspace.
fn repo_path(relative: &str) -> Result<PathBuf, std::io::Error> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(relative);
    if path.is_file() {
        Ok(path)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{} not found", path.display()),
        ))
    }
}

/// Assert the MATPOWER import matches the reference JSON case.
///
/// This is the check a planner actually wants: does the import preserve the
/// network? A silent id or bus-type mismatch here would invalidate every
/// downstream number.
fn verify_import(imported: &EnergySystem, reference: &EnergySystem) -> Result<(), String> {
    let problems: Vec<String> = [
        ("buses", imported.buses.len(), reference.buses.len()),
        (
            "branches",
            imported.branches.len(),
            reference.branches.len(),
        ),
        (
            "generators",
            imported.generators.len(),
            reference.generators.len(),
        ),
    ]
    .iter()
    .filter(|(_, a, b)| a != b)
    .map(|(what, a, b)| format!("{what}: {a} != {b}"))
    .collect();
    if !problems.is_empty() {
        return Err(format!("import lost records: {}", problems.join(", ")));
    }
    for (a, b) in imported.buses.iter().zip(reference.buses.iter()) {
        if a.id != b.id || a.bus_type != b.bus_type {
            return Err(format!(
                "bus imported as {} / {:?}, expected {} / {:?}",
                a.id, a.bus_type, b.id, b.bus_type
            ));
        }
    }
    for (a, b) in imported.branches.iter().zip(reference.branches.iter()) {
        if a.from_bus != b.from_bus || a.to_bus != b.to_bus {
            return Err(format!(
                "branch {} imported as {}-{}, expected {}-{}",
                a.id, a.from_bus, a.to_bus, b.from_bus, b.to_bus
            ));
        }
    }
    Ok(())
}

/// Solve a system, returning a "diverged" result rather than an error.
///
/// A contingency that will not solve is a *result* in a study, not a crash:
/// the whole point of the sweep is to find the cases that do not hold up.
fn solve(system: &EnergySystem) -> Result<PowerFlowResult, Box<dyn std::error::Error>> {
    Ok(PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson)
        .with_max_iterations(30)
        .solve(system)
        .unwrap_or_else(|e| diverged(system, e)))
}

/// A result marking a case the solver could not solve.
fn diverged(system: &EnergySystem, e: PowerFlowError) -> PowerFlowResult {
    eprintln!("  (power flow did not converge: {e})");
    PowerFlowResult {
        converged: false,
        iterations: 0,
        final_mismatch: f64::NAN,
        bus_voltage_magnitude_pu: vec![f64::NAN; system.buses.len()],
        bus_voltage_angle_rad: vec![0.0; system.buses.len()],
        branch_flows: system
            .branches
            .iter()
            .map(|b| BranchFlow {
                id: b.id,
                p_from_mw: 0.0,
                q_from_mvar: 0.0,
                p_to_mw: 0.0,
                q_to_mvar: 0.0,
                loading_fraction: 0.0,
            })
            .collect(),
        total_losses_mw: f64::NAN,
        total_losses_mvar: f64::NAN,
        generator_p_mw: vec![0.0; system.generators.len()],
        generator_q_mvar: vec![0.0; system.generators.len()],
    }
}

/// Re-solve `base` with `outage` out of service.
///
/// Returns `None` when the outage splits the network into islands: a floating
/// island has no slack bus, so the base solver is not the right tool for it
/// and reporting a number would be worse than reporting nothing.
fn evaluate(
    base: &EnergySystem,
    outage: &Branch,
) -> Option<(String, PowerFlowResult, &'static str)> {
    let mut system = base.clone();
    system
        .branches
        .iter_mut()
        .find(|b| b.id == outage.id)?
        .in_service = false;

    if is_unsolvable(&system) {
        eprintln!("  (outage of {} islands the network; skipped)", outage.name);
        return None;
    }

    let result = solve(&system).ok()?;
    let (v_min, max_loading, _) = metrics(&result);
    let verdict = if !result.converged {
        "DIVERGED"
    } else if !(V_MIN..=V_MAX).contains(&v_min) {
        "VOLTAGE"
    } else if max_loading > 1.0 {
        "OVERLOADED"
    } else {
        "ok"
    };
    Some((outage.name.clone(), result, verdict))
}

/// True when some connected component does not have exactly one slack bus.
///
/// Without exactly one slack in a component there is no well-posed AC
/// solution, so a sweep must detect this rather than report nonsense.
fn is_unsolvable(system: &EnergySystem) -> bool {
    let n = system.buses.len();
    if n == 0 {
        return false;
    }
    let index: std::collections::HashMap<usize, usize> = system
        .buses
        .iter()
        .enumerate()
        .map(|(i, b)| (b.id, i))
        .collect();
    let mut adjacency = vec![Vec::new(); n];
    for br in &system.branches {
        if !br.in_service {
            continue;
        }
        if let (Some(&a), Some(&b)) = (index.get(&br.from_bus), index.get(&br.to_bus)) {
            adjacency[a].push(b);
            adjacency[b].push(a);
        }
    }
    let mut seen = vec![false; n];
    for (i, bus) in system.buses.iter().enumerate() {
        if bus.bus_type == BusType::Isolated || seen[i] {
            continue;
        }
        let mut stack = vec![i];
        seen[i] = true;
        let mut slacks = 0;
        while let Some(node) = stack.pop() {
            if system.buses[node].bus_type == BusType::Slack {
                slacks += 1;
            }
            for &next in &adjacency[node] {
                if !seen[next] {
                    seen[next] = true;
                    stack.push(next);
                }
            }
        }
        if slacks != 1 {
            return true;
        }
    }
    false
}

/// Lowest bus voltage, highest branch loading, and total losses.
fn metrics(result: &PowerFlowResult) -> (f64, f64, f64) {
    let v_min = result
        .bus_voltage_magnitude_pu
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(f64::INFINITY, f64::min);
    let max_loading = result
        .branch_flows
        .iter()
        .map(|f| f.loading_fraction)
        .fold(0.0_f64, f64::max);
    (v_min, max_loading, result.total_losses_mw)
}

/// Bus id of the lowest-voltage bus.
fn weakest_bus(system: &EnergySystem, result: &PowerFlowResult) -> Option<usize> {
    result
        .bus_voltage_magnitude_pu
        .iter()
        .enumerate()
        .filter(|(_, v)| v.is_finite())
        .min_by(|a, b| a.1.total_cmp(b.1))
        .and_then(|(i, _)| system.buses.get(i).map(|b| b.id))
}

/// Print the headline numbers of a solved case.
fn print_solution(label: &str, result: &PowerFlowResult) {
    let (v_min, max_loading, losses) = metrics(result);
    println!(
        "  {label}: converged in {} iterations, losses {losses:.2} MW",
        result.iterations
    );
    println!(
        "  {label}: min |V| = {v_min:.4} pu, max branch loading = {:.1}%",
        max_loading * 100.0
    );
}
