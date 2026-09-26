//! PSS/E RAW data-file import and export.
//!
//! A RAW file is a sequence of named blocks:
//!
//! ```text
//! BEGIN SYSTEM DATA
//! CASEID 33 1.10 33 60 / 100.0 33 100.0 /
//! END SYSTEM DATA
//! BEGIN BUS DATA
//!  101 ELM CREEK 230.00 3 1 1 1 1.0200 0.00000 / 1 1 1 1 250.0 100.0 /
//! END BUS DATA
//! END
//! ```
//!
//! Records wrap at column 80 and are delimited by `/`; the fields *within* a
//! delimiter are whitespace separated. A record can therefore span several
//! physical lines, and a line can contain several delimiters.
//!
//! ## Supported dialect
//!
//! The parser reads the leading, positionally stable fields of the v30, v32,
//! and v33 layouts and ignores any trailing field it does not need. Later
//! versions of PSS/E only *append* to the end of a data block, so reading a
//! prefix is forward-compatible; writing targets v33.
//!
//! | Block     | Fields read                                                    |
//! |-----------|---------------------------------------------------------------|
//! | `SYSTEM`  | `CASEID`, `BASFRQ`, `SBASE`                                    |
//! | `BUS`     | `I`, `NAME`, `BASKV`, `IDE`, `AREA`, `ZONE`, `VMAG`, `VA`, plus the record's load `PL`/`QL` |
//! | `BRANCH`  | `I`, `J`, `CKT`, `R`, `X`, `B`, `RATEA`, `ST`                  |
//! | `GENERATOR`| `I`, `ID`, `PG`, `QG`, `QT`, `QB`, `VS`, `STAT`, `PT`, `PB`  |
//! | `LOAD`    | `I`, `ID`, `STATUS`, `PL`, `IQ`                               |

use std::collections::HashMap;
use std::fmt::Write as _;

use tpt_nrg_core::{Branch, Bus, BusType, EnergySystem, Generator, GeneratorType, Load};

use crate::error::{field_index, format_number, InteropError, InteropResult};

/// Degrees-to-radians conversion factor.
const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;

/// Candidate sub-block counts per bus record, newest dialect first.
///
/// A bus record is `/`-delimited into: the bus fields, the load, the fixed
/// shunt, and up to three generator entries. PSS/E only ever *appends* to a
/// record, so the per-record sub-block count is what identifies the dialect.
/// Trying the candidates largest-first and keeping the one that divides the
/// block exactly identifies v33 (6), v32 (5), v30 (4), and v29 (3).
const BUS_SUBDIVISIONS: [usize; 4] = [6, 5, 4, 3];

/// Minimum fields in the leading sub-block of a bus record: `I NAME BASKV IDE
/// AREA ZONE OWNER VMAG VA` plus the two voltage-limit fields that every
/// supported dialect carries.
const BUS_FIELDS_MIN: usize = 10;

/// Map a PSS/E bus type code (`IDE`) to a [`BusType`].
///
/// 1 = load, 2 = generator, 3 = slack, 4 = isolated.
fn bus_type_from_psse(code: usize) -> InteropResult<BusType> {
    match code {
        1 => Ok(BusType::Pq),
        2 => Ok(BusType::Pv),
        3 => Ok(BusType::Slack),
        4 => Ok(BusType::Isolated),
        other => Err(InteropError::bad_value(
            "psse",
            "bus IDE",
            format!("unknown PSS/E bus type {other}"),
        )),
    }
}

/// Map a [`BusType`] back to a PSS/E `IDE` code.
fn bus_type_to_psse(t: BusType) -> usize {
    match t {
        BusType::Pq => 1,
        BusType::Pv => 2,
        BusType::Slack => 3,
        BusType::Isolated => 4,
    }
}

/// One `/`-delimited sub-block of a RAW file, split into field tokens.
type SubBlock = Vec<String>;

/// All `BEGIN <name> DATA` blocks of a RAW file, keyed by upper-case name.
///
/// Each block holds its sub-blocks in file order; records are recovered by
/// [`group_records`], because the number of sub-blocks per record depends on
/// the dialect.
#[derive(Debug, Default)]
struct RawSections {
    /// Section name to its ordered sub-blocks.
    map: HashMap<String, Vec<SubBlock>>,
}

impl RawSections {
    /// Return the sub-blocks of a section, or an empty slice.
    fn get(&self, name: &str) -> &[SubBlock] {
        self.map.get(name).map_or(&[], Vec::as_slice)
    }
}

/// Split a RAW file into its data blocks.
fn parse_sections(text: &str) -> RawSections {
    let mut sections = RawSections::default();
    let mut current: Option<String> = None;
    for raw in text.lines() {
        let line = strip_comment(raw);
        if line.is_empty() {
            continue;
        }
        let upper = line.to_ascii_uppercase();
        if let Some(name) = block_header(&upper) {
            current = Some(name);
            continue;
        }
        let Some(name) = current.clone() else { continue };
        if upper.starts_with("END") {
            current = None;
            continue;
        }
        // A trailing `/` terminates the record, so the empty piece after it
        // is dropped; interior empty pieces are kept, because PSS/E uses
        // `/ /` placeholders whose *position* carries the dialect.
        let mut pieces: Vec<&str> = line.split('/').collect();
        if pieces.last().is_some_and(|p| p.trim().is_empty()) {
            pieces.pop();
        }
        for piece in pieces {
            let tokens: Vec<String> = piece.split_whitespace().map(str::to_string).collect();
            sections
                .map
                .entry(name.clone())
                .or_default()
                .push(tokens);
        }
    }
    sections
}

/// Regroup sub-blocks into records of `per_record` sub-blocks each.
fn group_records(sub_blocks: &[SubBlock], per_record: usize) -> Vec<Vec<SubBlock>> {
    if per_record == 0 {
        return Vec::new();
    }
    sub_blocks
        .chunks(per_record)
        .map(<[SubBlock]>::to_vec)
        .filter(|r| r.len() == per_record)
        .collect()
}

/// Infer how many sub-blocks make up one bus record.
///
/// Divisibility alone is ambiguous (a v29 file with an even number of buses
/// also divides by 3 and 6), so each candidate is additionally required to
/// produce records whose first sub-block looks like a bus: at least
/// [`BUS_FIELDS_MIN`] fields, a positive integer bus number, and a base kV.
fn bus_record_size(sub_blocks: &[SubBlock]) -> usize {
    for candidate in BUS_SUBDIVISIONS {
        if sub_blocks.is_empty() || sub_blocks.len() % candidate != 0 {
            continue;
        }
        let records = group_records(sub_blocks, candidate);
        if !records.is_empty() && records.iter().all(|r| looks_like_bus(&r[0])) {
            return candidate;
        }
    }
    1
}

/// True if a sub-block has the shape of the leading sub-block of a bus record.
fn looks_like_bus(fields: &[String]) -> bool {
    if fields.len() < BUS_FIELDS_MIN {
        return false;
    }
    idx(fields, 0) > 0 && opt_num(fields, 2) > 0.0
}

/// Recognize `BEGIN <NAME> DATA` and return the block name.
fn block_header(upper: &str) -> Option<String> {
    let tokens: Vec<&str> = upper.split_whitespace().collect();
    if tokens.len() >= 3 && tokens[0] == "BEGIN" && tokens[2] == "DATA" {
        Some(tokens[1].to_string())
    } else {
        None
    }
}

/// Strip a PSS/E comment (`//` to end of line) and trim.
fn strip_comment(line: &str) -> &str {
    match line.find("//") {
        Some(i) => line[..i].trim(),
        None => line.trim(),
    }
}

/// Parse a PSS/E numeric field, tolerating Fortran-style `D` exponents
/// (`1.5D+02` as well as `1.5E+02`).
fn parse_field(raw: &str) -> Option<f64> {
    raw.replace(['D', 'd'], "E").parse::<f64>().ok()
}

/// Read a field as a bus/branch index, or `0` when absent or non-positive.
fn idx(fields: &[String], i: usize) -> usize {
    field_index(opt_num(fields, i))
}

/// Read a field as `f64`, defaulting to `0.0` when absent or unparseable.
fn opt_num(fields: &[String], i: usize) -> f64 {
    fields.get(i).and_then(|s| parse_field(s)).unwrap_or(0.0)
}

/// Read a field as a string, stripping PSS/E single quotes.
fn text(fields: &[String], i: usize, fallback: &str) -> String {
    let raw = fields.get(i).map_or("", String::as_str);
    let cleaned = raw.trim().trim_matches('\'').trim();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned.to_string()
    }
}

/// Parse a PSS/E RAW file into an [`EnergySystem`].
///
/// Buses come from `BUS DATA`, branches from `BRANCH DATA`, generators from
/// `GENERATOR DATA` (falling back to units implied by the bus `IDE` when the
/// block is absent), and loads from `LOAD DATA` plus the per-bus load
/// sub-block.
///
/// # Errors
///
/// Returns [`InteropError::Missing`] if `BUS DATA` is absent,
/// [`InteropError::BadValue`] if a field cannot be interpreted, and
/// [`InteropError::InvalidSystem`] if the result fails structural validation.
pub fn from_psse(text: &str) -> InteropResult<EnergySystem> {
    let sections = parse_sections(text);
    let bus_blocks = sections.get("BUS");
    if bus_blocks.is_empty() {
        return Err(InteropError::missing("psse", "BEGIN BUS DATA block"));
    }
    let (base_mva, frequency_hz, case_id) = read_system_data(sections.get("SYSTEM"));
    let system_id = sanitize_id(&case_id);

    let mut sys = EnergySystem::new(
        system_id,
        format!("{case_id} (PSS/E)"),
        base_mva,
        frequency_hz,
    );

    for record in group_records(bus_blocks, bus_record_size(bus_blocks)) {
        let bus_fields = &record[0];
        let load_fields = record.get(1).cloned().unwrap_or_default();
        let bus = psse_bus_to_core(bus_fields, &load_fields)?;
        sys.add_bus(bus).map_err(InteropError::InvalidSystem)?;
    }

    for (i, fields) in single_sub_blocks(sections.get("BRANCH")).into_iter().enumerate() {
        let branch = psse_branch_to_core(i + 1, &fields)?;
        sys.add_branch(branch)
            .map_err(InteropError::InvalidSystem)?;
    }

    if sections.get("GENERATOR").is_empty() {
        add_implicit_generators(&mut sys)?;
    } else {
        for (i, fields) in single_sub_blocks(sections.get("GENERATOR"))
            .into_iter()
            .enumerate()
        {
            let gen = psse_gen_to_core(i + 1, &fields)?;
            sys.add_generator(gen)
                .map_err(InteropError::InvalidSystem)?;
        }
    }

    for (i, fields) in single_sub_blocks(sections.get("LOAD")).into_iter().enumerate() {
        let load = psse_load_to_core(i + 1, &fields)?;
        sys.add_load(load).map_err(InteropError::InvalidSystem)?;
    }

    sys.metadata.insert("source".into(), serde_json::json!("PSS/E"));
    sys.metadata.insert("base_mva".into(), serde_json::json!(base_mva));
    sys.validate().map_err(InteropError::InvalidSystem)?;
    Ok(sys)
}

/// Flatten single-sub-block records into their field lists.
fn single_sub_blocks(blocks: &[SubBlock]) -> Vec<SubBlock> {
    group_records(blocks, 1).into_iter().map(|r| r[0].clone()).collect()
}

/// Read `SBASE`, `BASFRQ`, and the case identifier from `SYSTEM DATA`.
///
/// `SYSTEM DATA` is two single-sub-block records: the first holds
/// `CASEID REV XFRRAT NXFRAT BASFRQ`, the second `SBASE REV ZBASKV ...`.
fn read_system_data(blocks: &[SubBlock]) -> (f64, f64, String) {
    let mut base_mva = 100.0;
    let mut frequency_hz = 60.0;
    let mut case_id = "psse".to_string();
    for block in blocks {
        if block.len() >= 5 {
            case_id = text(block, 0, "psse");
            frequency_hz = opt_num(block, 4).abs();
        } else if !block.is_empty() {
            let sbase = opt_num(block, 0);
            if sbase > 0.0 {
                base_mva = sbase;
            }
        }
    }
    if frequency_hz < 1.0 {
        frequency_hz = 60.0;
    }
    (base_mva, frequency_hz, case_id)
}

/// Reduce a PSS/E case identifier to a safe system id.
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
        "psse".to_string()
    } else {
        trimmed
    }
}

/// Convert a PSS/E bus sub-block (plus its load sub-block) to a [`Bus`].
///
/// Fields: `I NAME BASKV IDE AREA ZONE OWNER VMAG VA NVHI NVLO`.
fn psse_bus_to_core(bus: &[String], load: &[String]) -> InteropResult<Bus> {
    let id = idx(bus, 0);
    if id == 0 {
        return Err(InteropError::bad_value(
            "psse",
            "bus I",
            "must be a positive integer",
        ));
    }
    let name = text(bus, 1, &format!("BUS {id}"));
    let raw_kv = opt_num(bus, 2);
    let base_kv = if raw_kv > 0.0 { raw_kv } else { 100.0 };
    Ok(
        Bus::new(id, name, bus_type_from_psse(idx(bus, 3))?)
            .with_voltage_pu(opt_num(bus, 7), opt_num(bus, 8) * DEG_TO_RAD)
            .with_base_kv(base_kv)
            .with_load(opt_num(load, 4), opt_num(load, 5)),
    )
}

/// Convert a PSS/E branch record to a [`Branch`].
///
/// Fields: `I J CKT R X B RATEA RATEB RATEC GI BI GJ BJ ST`.
fn psse_branch_to_core(id: usize, f: &[String]) -> InteropResult<Branch> {
    let (from_bus, to_bus) = (idx(f, 0), idx(f, 1));
    if from_bus == 0 || to_bus == 0 {
        return Err(InteropError::bad_value(
            "psse",
            "branch endpoints",
            "I and J must be positive integers",
        ));
    }
    let ckt = text(f, 2, "1");
    let rate_a = opt_num(f, 6);
    let rating = if rate_a > 0.0 { rate_a } else { 100.0 };
    // Transformer taps live in the branch *winding* records, which this
    // subset does not read, so a unity tap is the correct line default.
    let in_service = if f.len() > 13 { opt_num(f, 13) != 0.0 } else { true };
    Ok(
        Branch::new(
            id,
            format!("{from_bus}-{to_bus}-{ckt}"),
            from_bus,
            to_bus,
            opt_num(f, 3),
            opt_num(f, 4),
        )
        .with_susceptance(opt_num(f, 5))
        .with_rating(rating)
        .with_in_service(in_service),
    )
}

/// Convert a PSS/E generator record to a [`Generator`].
///
/// Fields: `I ID PG QG QT QB VS MBASE ZR ZX RT XT GTAP STAT RMPCT PT PB`.
/// PSS/E has no technology column, so units are typed
/// [`GeneratorType::Thermal`].
fn psse_gen_to_core(id: usize, f: &[String]) -> InteropResult<Generator> {
    let bus_id = idx(f, 0);
    if bus_id == 0 {
        return Err(InteropError::bad_value(
            "psse",
            "generator I",
            "must be a positive integer",
        ));
    }
    let in_service = if f.len() > 13 { opt_num(f, 13) != 0.0 } else { true };
    let raw_vs = opt_num(f, 6);
    let mut gen = Generator::new(
        id,
        format!("G{id}"),
        GeneratorType::Thermal,
        opt_num(f, 15),
        opt_num(f, 16),
    )
    .at_bus(bus_id)
    .with_p_schedule(if in_service { opt_num(f, 2) } else { 0.0 })
    .with_voltage_setpoint(if raw_vs > 0.0 { raw_vs } else { 1.0 })
    .with_reactive_limits(opt_num(f, 4), opt_num(f, 3));
    gen.in_service = in_service;
    Ok(gen)
}

/// Convert a PSS/E load record to a [`Load`].
///
/// Fields: `I ID STATUS AREA ZONE PL IPL IQ LIQ YP YQ`.
fn psse_load_to_core(id: usize, f: &[String]) -> InteropResult<Load> {
    let bus_id = idx(f, 0);
    if bus_id == 0 {
        return Err(InteropError::bad_value(
            "psse",
            "load I",
            "must be a positive integer",
        ));
    }
    let in_service = if f.len() > 2 { opt_num(f, 2) != 0.0 } else { true };
    let mut load = Load::new(
        id,
        format!("L{id}"),
        bus_id,
        opt_num(f, 5),
        opt_num(f, 7),
    );
    load.in_service = in_service;
    Ok(load)
}

/// Synthesize generators for generator buses when `GENERATOR DATA` is absent.
///
/// PSS/E models without a machine block still mark generator buses with
/// `IDE = 2`/`3`; those become thermal units sized from the bus base voltage.
fn add_implicit_generators(sys: &mut EnergySystem) -> InteropResult<()> {
    let buses: Vec<(usize, f64)> = sys
        .buses
        .iter()
        .filter(|b| matches!(b.bus_type, BusType::Pv | BusType::Slack))
        .map(|b| (b.id, b.base_kv))
        .collect();
    for (i, (bus_id, base_kv)) in buses.into_iter().enumerate() {
        let id = i + 1;
        let p_max = (base_kv * 0.5).max(10.0);
        sys.add_generator(
            Generator::new(id, format!("G{id}"), GeneratorType::Thermal, p_max, 0.0)
                .at_bus(bus_id),
        )
        .map_err(InteropError::InvalidSystem)?;
    }
    Ok(())
}

/// Format a number for a fixed-column RAW field, or a blank when zero.
///
/// PSS/E treats an omitted field as zero, so blanking zeros keeps exported
/// files the width PSS/E users expect.
fn raw_field(v: f64) -> String {
    if v == 0.0 {
        String::new()
    } else {
        format_number(v, 5)
    }
}

/// Truncate and upper-case a name to the 12 characters PSS/E allows.
fn psse_name(name: &str, fallback: &str) -> String {
    let source = if name.trim().is_empty() { fallback } else { name };
    let cleaned: String = source
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { ' ' })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.chars().take(12).collect()
    }
}

/// Serialize an [`EnergySystem`] as a PSS/E v33 RAW file.
///
/// Per-bus loads are written into the per-bus load sub-block, which is where
/// PSS/E itself keeps them.
///
/// # Errors
///
/// Returns [`InteropError::BadValue`] if the system contains a non-finite
/// number or a bus name that cannot be encoded.
pub fn to_psse(system: &EnergySystem) -> InteropResult<String> {
    for b in &system.buses {
        if !b.voltage_magnitude_pu.is_finite() || !b.load_mw.is_finite() {
            return Err(InteropError::bad_value(
                "psse",
                format!("bus {}", b.id),
                "non-finite value cannot be written to a RAW file",
            ));
        }
    }
    let mut out = String::with_capacity(4096);
    let case_id = psse_name(&system.id, "CASE");

    let _ = writeln!(out, "BEGIN SYSTEM DATA");
    let _ = writeln!(out, "{case_id} 33 1.10 33 {} /", raw_field(system.frequency_hz));
    let _ = writeln!(out, "{} 33 100.00 /", raw_field(system.base_mva));
    let _ = writeln!(out, "END SYSTEM DATA");

    let _ = writeln!(out, "BEGIN BUS DATA");
    for b in &system.buses {
        // v33 bus record: the bus fields, then the load sub-block, the fixed
        // shunt sub-block, and three machine sub-blocks (only the first of
        // which this crate uses). Emitting all six keeps the record count
        // consistent with the dialect detection in `bus_record_size`.
        let _ = writeln!(
            out,
            "{:>7} {:<12} {:>7.2} {:>2} {:>3} {:>3} {:>3} {:>9.5} {:>10.5} / \
             1 1 1 1 {} {} / / / / / /",
            b.id,
            psse_name(&b.name, &format!("BUS {}", b.id)),
            b.base_kv,
            bus_type_to_psse(b.bus_type),
            1,
            1,
            1,
            b.voltage_magnitude_pu,
            b.voltage_angle_rad / DEG_TO_RAD,
            raw_field(b.load_mw),
            raw_field(b.load_mvar)
        );
    }
    let _ = writeln!(out, "END BUS DATA");

    let _ = writeln!(out, "BEGIN BRANCH DATA");
    for br in &system.branches {
        let ckt = br
            .name
            .rsplit_once('-')
            .map_or("1", |(_, c)| c);
        let _ = writeln!(
            out,
            "{:>7} {:>7} {:<2} {:>10.5} {:>10.5} {:>10.5} {:>6.0} {:>6.0} {:>6.0} /",
            br.from_bus,
            br.to_bus,
            ckt,
            br.resistance_pu,
            br.reactance_pu,
            br.susceptance_pu,
            br.rating_mva,
            br.rating_mva,
            br.rating_mva
        );
    }
    let _ = writeln!(out, "END BRANCH DATA");

    let _ = writeln!(out, "BEGIN GENERATOR DATA");
    for g in &system.generators {
        // Machine block: I ID PG QG QT QB VS MBASE / ZR ZX RT XT GTAP STAT
        // RMPCT PT PB.
        let _ = writeln!(
            out,
            "{:>7} {:<2} {:>9.4} {:>9.4} {:>9.4} {:>9.4} {:>8.4} {:>7.0} / \
             {:>9.4} {:>9.4} {:>9.4} {:>9.4} {:>8.4} {:>2} {:>7.4} {:>9.4} {:>9.4} /",
            g.bus_id,
            "'1'",
            g.p_schedule_mw,
            0.0,
            g.q_max_mvar,
            g.q_min_mvar,
            g.voltage_setpoint_pu,
            100.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            u8::from(g.in_service),
            100.0,
            g.p_max_mw,
            g.p_min_mw
        );
    }
    let _ = writeln!(out, "END GENERATOR DATA");

    let _ = writeln!(out, "BEGIN LOAD DATA");
    for l in &system.loads {
        let _ = writeln!(
            out,
            "{:>7} {:<2} {:>2} {:>3} {:>3} {:>9.2} {:>9.2} {:>9.2} {:>9.2} /",
            l.bus_id,
            "'1'",
            u8::from(l.in_service),
            1,
            1,
            l.p_mw,
            l.p_mw,
            l.q_mvar,
            l.q_mvar
        );
    }
    let _ = writeln!(out, "END LOAD DATA");
    let _ = writeln!(out, "END");
    Ok(out)
}
