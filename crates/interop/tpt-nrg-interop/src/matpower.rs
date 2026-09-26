//! MATPOWER case-file import and export.
//!
//! MATPOWER stores a case as a MATLAB script in which each data matrix is a
//! semicolon-terminated list of whitespace-separated numeric rows:
//!
//! ```text
//! mpc.baseMVA = 100;
//! %% bus data
//! %   bus_i  type  Pd  Qd  Gs  Bs  area  Vm  Va  baseKV  zone  Vmax  Vmin
//!     1  3  0  0  0  0  1  1.06  0  132  1  1.06  0.94;
//! ```
//!
//! Comments start at `%`. The parser is deliberately textual rather than a
//! MATLAB interpreter: it understands `mpc.<name> = <matrix|number|string>;`
//! assignments, which covers every published `case*.m` file, and rejects
//! anything else with [`InteropError::Parse`].

use std::collections::HashMap;
use std::fmt::Write as _;

use tpt_nrg_core::{Branch, Bus, BusType, EnergySystem, Generator, GeneratorType};

use crate::error::{field_index, format_number, InteropError, InteropResult};

/// Degrees-to-radians conversion factor.
const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;

/// A parsed `mpc.<name> = ...;` assignment.
#[derive(Debug, Clone)]
enum Assignment {
    /// A numeric matrix, one `Vec<f64>` per row.
    Matrix(Vec<Vec<f64>>),
    /// A single number.
    Number(f64),
    /// A single-quoted string.
    Text(String),
}

impl Assignment {
    /// Return the matrix rows, or an error if this is not a matrix.
    fn as_matrix(&self, name: &str) -> InteropResult<&[Vec<f64>]> {
        match self {
            Self::Matrix(rows) => Ok(rows),
            _ => Err(InteropError::parse(
                "matpower",
                format!("mpc.{name} is not a numeric matrix"),
            )),
        }
    }

    /// Return the scalar, or an error if this is not a number.
    fn as_number(&self, name: &str) -> InteropResult<f64> {
        match self {
            Self::Number(v) => Ok(*v),
            _ => Err(InteropError::parse(
                "matpower",
                format!("mpc.{name} is not a number"),
            )),
        }
    }
}

/// Strip a trailing `%` comment from a line and trim it.
fn strip_comment(line: &str) -> &str {
    match line.find('%') {
        Some(i) => line[..i].trim(),
        None => line.trim(),
    }
}

/// Read the `i`-th field of a row, or `0.0` when the row is short.
fn field(row: &[f64], i: usize) -> f64 {
    row.get(i).copied().unwrap_or(0.0)
}

/// Read the `i`-th field of a row as a bus index, or `0` when the row is
/// short or non-positive.
fn field_usize(row: &[f64], i: usize) -> usize {
    field_index(field(row, i))
}

/// Round to 10 decimal places so exports are stable across platforms.
fn round10(v: f64) -> f64 {
    (v * 1.0e10).round() / 1.0e10
}

/// Format a number the way MATPOWER expects: bare integers without a decimal
/// point, otherwise the shortest 10-decimal-rounded representation.
fn fmt_num(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    format_number(round10(v), 10)
}

/// Parse a MATPOWER `.m` case file into a map of assignments.
///
/// A matrix assignment opens with `mpc.<name> = [` and closes with `];`,
/// which may be many lines later, so the parser tracks whether it is inside a
/// matrix and accumulates rows until the closing bracket.
fn parse_assignments(text: &str) -> InteropResult<HashMap<String, Assignment>> {
    let mut out: HashMap<String, Assignment> = HashMap::new();
    let mut open_matrix: Option<String> = None;
    let mut rows: Vec<Vec<f64>> = Vec::new();

    for raw in text.lines() {
        let line = strip_comment(raw);
        if line.is_empty() {
            continue;
        }
        if let Some(name) = open_matrix.clone() {
            let (chunk, closes) = split_matrix_chunk(line);
            rows.extend(parse_matrix_body(chunk));
            if closes {
                out.insert(name, Assignment::Matrix(std::mem::take(&mut rows)));
                open_matrix = None;
            }
            continue;
        }
        if let Some((name, raw)) = parse_assignment_line(line) {
            match raw.strip_prefix('[') {
                // A multi-line matrix literal: `mpc.<name> = [` ... `];`.
                Some(rest) => {
                    let (chunk, closes) = split_matrix_chunk(rest);
                    rows = parse_matrix_body(chunk);
                    if closes {
                        out.insert(name, Assignment::Matrix(std::mem::take(&mut rows)));
                    } else {
                        open_matrix = Some(name);
                    }
                }
                None => {
                    out.insert(name, parse_value(&raw));
                }
            }
        }
    }

    if let Some(name) = open_matrix {
        return Err(InteropError::parse(
            "matpower",
            format!("mpc.{name} is never closed with `];`"),
        ));
    }
    if out.is_empty() {
        return Err(InteropError::parse(
            "matpower",
            "no `mpc.<name> = ...;` assignments found; is this a MATPOWER case file?",
        ));
    }
    Ok(out)
}

/// Split a line into the part inside the matrix and whether it closes it.
fn split_matrix_chunk(line: &str) -> (&str, bool) {
    match line.find(']') {
        Some(i) => (&line[..i], true),
        None => (line, false),
    }
}

/// Parse a single `mpc.<name> = <value>;` line, returning the name and the
/// raw (unparsed) value so the caller can detect a multi-line matrix literal.
///
/// The terminator is either the `;` that ends a single-line value or the `[`
/// that opens a multi-line matrix, which has no `;` of its own.
fn parse_assignment_line(line: &str) -> Option<(String, String)> {
    let eq = line.find('=')?;
    let lhs = line[..eq].trim();
    let name = lhs.strip_prefix("mpc.")?.trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let rhs = line[eq + 1..].trim();
    let value = if rhs.starts_with('[') {
        rhs
    } else {
        rhs.strip_suffix(';')?.trim()
    };
    Some((name.to_string(), value.to_string()))
}

/// Parse the right-hand side of an assignment.
fn parse_value(value: &str) -> Assignment {
    if let Some(inner) = single_quoted(value) {
        return Assignment::Text(inner);
    }
    if value.starts_with('[') || value.contains(';') {
        return Assignment::Matrix(parse_matrix_body(value));
    }
    match value.parse::<f64>() {
        Ok(v) => Assignment::Number(v),
        // Non-numeric right-hand sides are preserved as text and ignored.
        Err(_) => Assignment::Text(value.to_string()),
    }
}

/// Return the contents of a single-quoted MATLAB string literal.
fn single_quoted(value: &str) -> Option<String> {
    let v = value.trim();
    let inner = v.strip_prefix('\'')?.strip_suffix('\'')?;
    Some(inner.to_string())
}

/// Parse a bracketed MATPOWER matrix body into rows of numbers.
fn parse_matrix_body(value: &str) -> Vec<Vec<f64>> {
    let body = value.trim().trim_start_matches('[').trim_end_matches(']');
    let mut rows = Vec::new();
    for row in body.split(';') {
        let mut parsed = Vec::new();
        for token in row.trim().split([',', ' ', '\t']) {
            let token = token.trim();
            if let Ok(v) = token.parse::<f64>() {
                parsed.push(v);
            }
        }
        if !parsed.is_empty() {
            rows.push(parsed);
        }
    }
    rows
}

/// Map a MATPOWER bus type code to a [`BusType`].
///
/// MATPOWER uses 1 = PQ, 2 = PV, 3 = reference (slack), 4 = isolated.
fn bus_type_from_matpower(code: usize) -> InteropResult<BusType> {
    match code {
        1 => Ok(BusType::Pq),
        2 => Ok(BusType::Pv),
        3 => Ok(BusType::Slack),
        4 => Ok(BusType::Isolated),
        other => Err(InteropError::bad_value(
            "matpower",
            "bus type",
            format!("unknown MATPOWER bus type {other}"),
        )),
    }
}

/// Map a [`BusType`] back to its MATPOWER code.
fn bus_type_to_matpower(t: BusType) -> usize {
    match t {
        BusType::Pq => 1,
        BusType::Pv => 2,
        BusType::Slack => 3,
        BusType::Isolated => 4,
    }
}

/// Reduce a MATPOWER case name to a filesystem- and URL-safe system id.
fn sanitize_id(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "matpower".to_string()
    } else {
        trimmed
    }
}

/// Fetch a required matrix assignment.
fn require_matrix<'a>(
    assignments: &'a HashMap<String, Assignment>,
    name: &str,
) -> InteropResult<&'a [Vec<f64>]> {
    match assignments.get(name) {
        Some(a) => a.as_matrix(name),
        None => Err(InteropError::missing("matpower", format!("mpc.{name}"))),
    }
}

/// Parse a MATPOWER case file into an [`EnergySystem`].
///
/// # Errors
///
/// Returns [`InteropError::Missing`] if `mpc.baseMVA`, `mpc.bus`, `mpc.gen`,
/// or `mpc.branch` is absent, [`InteropError::Parse`] if the text is not a
/// MATPOWER case, and [`InteropError::InvalidSystem`] if the result fails
/// structural validation.
pub fn from_matpower(text: &str) -> InteropResult<EnergySystem> {
    let assignments = parse_assignments(text)?;
    let base_mva = match assignments.get("baseMVA") {
        Some(a) => a.as_number("baseMVA")?,
        None => return Err(InteropError::missing("matpower", "mpc.baseMVA")),
    };
    let bus_rows = require_matrix(&assignments, "bus")?;
    let gen_rows = require_matrix(&assignments, "gen")?;
    let branch_rows = require_matrix(&assignments, "branch")?;

    let case_name = match assignments.get("caseName") {
        Some(Assignment::Text(s)) => s.clone(),
        _ => "matpower".to_string(),
    };
    let system_id = sanitize_id(&case_name);

    let mut sys = EnergySystem::new(
        system_id,
        format!("{case_name} (MATPOWER)"),
        base_mva,
        60.0,
    );

    for row in bus_rows {
        sys.add_bus(matpower_bus_to_core(row)?)
            .map_err(InteropError::InvalidSystem)?;
    }
    for (i, row) in branch_rows.iter().enumerate() {
        // MATPOWER omits the status column in older cases, which means "in
        // service"; a present status of 0 means out of service.
        let in_service = row.len() <= 10 || field(row, 10) != 0.0;
        let branch = matpower_branch_to_core(i + 1, row, in_service)?;
        sys.add_branch(branch)
            .map_err(InteropError::InvalidSystem)?;
    }
    for (i, row) in gen_rows.iter().enumerate() {
        sys.add_generator(matpower_gen_to_core(i + 1, row)?)
            .map_err(InteropError::InvalidSystem)?;
    }

    sys.metadata
        .insert("source".into(), serde_json::json!("MATPOWER"));
    sys.metadata.insert("base_mva".into(), serde_json::json!(base_mva));
    sys.validate().map_err(InteropError::InvalidSystem)?;
    Ok(sys)
}

/// Convert a MATPOWER bus row to a [`Bus`].
///
/// Column layout: `bus_i type Pd Qd Gs Bs area Vm Va baseKV zone Vmax Vmin`.
fn matpower_bus_to_core(row: &[f64]) -> InteropResult<Bus> {
    let id = field_usize(row, 0);
    if id == 0 {
        return Err(InteropError::bad_value(
            "matpower",
            "bus_i",
            "must be a positive integer",
        ));
    }
    let base_kv = if field(row, 9) > 0.0 { field(row, 9) } else { 100.0 };
    Ok(
        Bus::new(id, format!("Bus {id}"), bus_type_from_matpower(field_usize(row, 1))?)
            .with_voltage_pu(field(row, 7), field(row, 8) * DEG_TO_RAD)
            .with_base_kv(base_kv)
            .with_load(field(row, 2), field(row, 3))
            .with_shunt(field(row, 4), field(row, 5)),
    )
}

/// Convert a MATPOWER branch row to a [`Branch`].
///
/// Column layout: `fbus tbus r x b rateA rateB rateC ratio angle status`.
fn matpower_branch_to_core(id: usize, row: &[f64], in_service: bool) -> InteropResult<Branch> {
    let from_bus = field_usize(row, 0);
    let to_bus = field_usize(row, 1);
    if from_bus == 0 || to_bus == 0 {
        return Err(InteropError::bad_value(
            "matpower",
            "branch endpoints",
            "fbus and tbus must be positive integers",
        ));
    }
    let rate_a = field(row, 5);
    let rating = if rate_a > 0.0 { rate_a } else { 100.0 };
    Ok(
        Branch::new(
            id,
            format!("{from_bus}-{to_bus}"),
            from_bus,
            to_bus,
            field(row, 2),
            field(row, 3),
        )
        .with_susceptance(field(row, 4))
        .with_tap(field(row, 8), field(row, 9) * DEG_TO_RAD)
        .with_rating(rating)
        .with_in_service(in_service),
    )
}

/// Convert a MATPOWER generator row to a [`Generator`].
///
/// Column layout: `bus Pg Qg Qmax Qmin Vg mBase status Pmax Pmin`. MATPOWER
/// has no technology column, so units are typed [`GeneratorType::Thermal`],
/// the conventional default for every published IEEE case.
fn matpower_gen_to_core(id: usize, row: &[f64]) -> InteropResult<Generator> {
    let bus_id = field_usize(row, 0);
    if bus_id == 0 {
        return Err(InteropError::bad_value(
            "matpower",
            "gen bus",
            "must be a positive integer",
        ));
    }
    let status = if row.len() > 7 { field(row, 7) } else { 1.0 };
    let v_setpoint = if field(row, 5) > 0.0 { field(row, 5) } else { 1.0 };
    let mut gen = Generator::new(
        id,
        format!("G{id}"),
        GeneratorType::Thermal,
        field(row, 8),
        field(row, 9),
    )
    .at_bus(bus_id)
    .with_p_schedule(field(row, 1))
    .with_voltage_setpoint(v_setpoint)
    .with_reactive_limits(field(row, 4), field(row, 3));
    gen.in_service = status != 0.0;
    Ok(gen)
}

/// Serialize an [`EnergySystem`] as a MATPOWER case file.
///
/// The output is MATLAB-valid: every matrix is a bracketed, semicolon-
/// terminated list of rows, and `mpc.baseMVA` carries the per-unit base.
/// Values are rounded to 10 decimals so that a file written on one platform
/// and re-read on another is bit-identical.
///
/// # Errors
///
/// Returns [`InteropError::BadValue`] if the system contains a non-finite
/// number, which a MATPOWER file cannot represent.
pub fn to_matpower(system: &EnergySystem) -> InteropResult<String> {
    check_finite(system)?;
    let mut out = String::with_capacity(4096);
    let case = sanitize_id(&system.id);
    let _ = writeln!(out, "function mpc = {case}");
    let _ = writeln!(out, "%% MATPOWER Case Format : Version 2");
    let _ = writeln!(out, "%% Generated by tpt-nrg-interop");
    let _ = writeln!(out, "mpc.version = '2';");
    let _ = writeln!(out, "mpc.baseMVA = {};", fmt_num(system.base_mva));

    let _ = writeln!(out, "\n%% bus data");
    let _ = writeln!(
        out,
        "%\tbus_i\ttype\tPd\tQd\tGs\tBs\tarea\tVm\tVa\tbaseKV\tzone\tVmax\tVmin"
    );
    let _ = writeln!(out, "mpc.bus = [");
    for b in &system.buses {
        let _ = writeln!(
            out,
            "\t{}\t{}\t{}\t{}\t{}\t{}\t1\t{}\t{}\t{}\t1\t1.1\t0.9;",
            b.id,
            bus_type_to_matpower(b.bus_type),
            fmt_num(b.load_mw),
            fmt_num(b.load_mvar),
            fmt_num(b.shunt_conductance_pu),
            fmt_num(b.shunt_susceptance_pu),
            fmt_num(b.voltage_magnitude_pu),
            fmt_num(b.voltage_angle_rad / DEG_TO_RAD),
            fmt_num(b.base_kv)
        );
    }
    let _ = writeln!(out, "];");

    let _ = writeln!(out, "\n%% generator data");
    let _ = writeln!(out, "%\tbus\tPg\tQg\tQmax\tQmin\tVg\tmBase\tstatus\tPmax\tPmin");
    let _ = writeln!(out, "mpc.gen = [");
    for g in &system.generators {
        let _ = writeln!(
            out,
            "\t{}\t{}\t0\t{}\t{}\t{}\t100\t{}\t{}\t{};",
            g.bus_id,
            fmt_num(g.p_schedule_mw),
            fmt_num(g.q_max_mvar),
            fmt_num(g.q_min_mvar),
            fmt_num(g.voltage_setpoint_pu),
            u8::from(g.in_service),
            fmt_num(g.p_max_mw),
            fmt_num(g.p_min_mw)
        );
    }
    let _ = writeln!(out, "];");

    let _ = writeln!(out, "\n%% branch data");
    let _ = writeln!(
        out,
        "%\tfbus\ttbus\tr\tx\tb\trateA\trateB\trateC\tratio\tangle\tstatus"
    );
    let _ = writeln!(out, "mpc.branch = [");
    for br in &system.branches {
        let _ = writeln!(
            out,
            "\t{}\t{}\t{}\t{}\t{}\t{}\t0\t0\t{}\t{}\t{};",
            br.from_bus,
            br.to_bus,
            fmt_num(br.resistance_pu),
            fmt_num(br.reactance_pu),
            fmt_num(br.susceptance_pu),
            fmt_num(br.rating_mva),
            fmt_num(br.tap_ratio),
            fmt_num(br.phase_shift_rad / DEG_TO_RAD),
            u8::from(br.in_service)
        );
    }
    let _ = writeln!(out, "];");

    Ok(out)
}

/// Reject non-finite values, which a MATPOWER file cannot represent.
fn check_finite(system: &EnergySystem) -> InteropResult<()> {
    let check = |v: f64, what: &str| -> InteropResult<()> {
        if v.is_finite() {
            Ok(())
        } else {
            Err(InteropError::bad_value(
                "matpower",
                what.to_string(),
                format!("non-finite value {v}"),
            ))
        }
    };
    for b in &system.buses {
        check(b.voltage_magnitude_pu, "bus voltage")?;
        check(b.load_mw, "bus load")?;
        check(b.load_mvar, "bus reactive load")?;
    }
    for g in &system.generators {
        check(g.p_max_mw, "generator p_max")?;
        check(g.p_min_mw, "generator p_min")?;
        check(g.p_schedule_mw, "generator schedule")?;
    }
    for br in &system.branches {
        check(br.resistance_pu, "branch resistance")?;
        check(br.reactance_pu, "branch reactance")?;
        check(br.rating_mva, "branch rating")?;
    }
    Ok(())
}
