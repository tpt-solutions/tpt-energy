//! Structural comparison of two energy systems.
//!
//! This is the engine behind `tpt-nrg convert --diff` and
//! `tpt-nrg convert --round-trip`, used to validate that a case survives a
//! format migration without losing or altering a single field.
//!
//! The comparison is structural rather than textual: both systems are
//! serialized to JSON and walked together, so a MATPOWER file and a PSS/E file
//! of the same network compare field by field instead of line by line.
//!
//! Two deliberate choices make the output useful on real cases:
//!
//! - **Arrays of records are matched by `id`, not by position.** MATPOWER,
//!   PSS/E, and CIM do not all preserve record order, so a positional diff
//!   would report spurious differences after a round trip. Records are keyed
//!   by their `id` field whenever both sides carry one, and compared by
//!   position otherwise.
//! - **Floating-point fields compare with a tolerance.** The default is
//!   [`DEFAULT_TOLERANCE`], scaled by the magnitude of the values being
//!   compared, so a value that round-tripped through a fixed-width text format
//!   is not reported as changed.
//!
//! ## Example
//!
//! ```
//! use tpt_nrg_interop::{diff_text, Format};
//!
//! let original = r#"{
//!   "id": "case", "name": "Case", "base_mva": 100.0, "frequency_hz": 60.0,
//!   "buses": [
//!     {"id": 1, "name": "B1", "type": "Slack"},
//!     {"id": 2, "name": "B2", "type": "Pq", "load_mw": 10.0, "load_mvar": 5.0}
//!   ],
//!   "branches": [{"id": 1, "name": "L1", "from_bus": 1, "to_bus": 2,
//!                 "resistance_pu": 0.01, "reactance_pu": 0.1}],
//!   "generators": [{"id": 1, "name": "G1", "bus_id": 1, "type": "Thermal",
//!                   "p_max_mw": 100.0, "p_min_mw": 10.0}]
//! }"#;
//! assert!(diff_text(original, Format::Json, original, Format::Json)?.is_empty());
//!
//! let edited = original.replace(r#""load_mw": 10.0"#, r#""load_mw": 12.5"#);
//! let differences = diff_text(original, Format::Json, &edited, Format::Json)?;
//! assert_eq!(differences.len(), 1);
//! assert_eq!(differences[0].path(), "buses[id=2].load_mw");
//! # Ok::<(), tpt_nrg_interop::InteropError>(())
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde_json::Value;

use crate::{EnergySystem, Format, InteropError, InteropResult};

/// Default relative-plus-absolute tolerance for floating-point comparisons.
///
/// Text exchange formats (MATPOWER column format, PSS/E fixed fields, CSV)
/// round numbers through a decimal representation, so bit-exact equality is
/// the wrong test. Values are treated as equal when
/// `|a - b| <= tolerance * max(|a|, |b|, 1)`.
pub const DEFAULT_TOLERANCE: f64 = 1e-9;

/// How a path differs between the two systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DifferenceKind {
    /// The path is present only in the right-hand system.
    Added,
    /// The path is present only in the left-hand system.
    Removed,
    /// The path is present in both, with different values.
    Changed,
}

impl DifferenceKind {
    /// Stable lower-case slug.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Changed => "changed",
        }
    }
}

impl fmt::Display for DifferenceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single structural difference between two systems.
///
/// Paths are dotted with bracketed record keys, for example
/// `buses[id=4].voltage_magnitude_pu` or `generators[id=1].cost_curve`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    kind: DifferenceKind,
    path: String,
    left: String,
    right: String,
}

impl Difference {
    /// How the path differs.
    #[must_use]
    pub fn kind(&self) -> DifferenceKind {
        self.kind
    }

    /// Location of the difference, e.g. `buses[id=2].load_mw`.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Rendered value on the left-hand side, or `"<absent>"` when added.
    #[must_use]
    pub fn left(&self) -> &str {
        &self.left
    }

    /// Rendered value on the right-hand side, or `"<absent>"` when removed.
    #[must_use]
    pub fn right(&self) -> &str {
        &self.right
    }
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} -> {}", self.path, self.left, self.right)
    }
}

/// Compare two already-parsed systems with the default tolerance.
///
/// # Errors
///
/// Returns [`InteropError::Parse`] if either system cannot be serialized for
/// comparison.
pub fn diff_systems(left: &EnergySystem, right: &EnergySystem) -> InteropResult<Vec<Difference>> {
    diff_systems_with_tolerance(left, right, DEFAULT_TOLERANCE)
}

/// Compare two already-parsed systems with an explicit float `tolerance`.
///
/// # Errors
///
/// Returns [`InteropError::Parse`] if either system cannot be serialized for
/// comparison.
pub fn diff_systems_with_tolerance(
    left: &EnergySystem,
    right: &EnergySystem,
    tolerance: f64,
) -> InteropResult<Vec<Difference>> {
    Ok(diff_values(&to_value(left)?, &to_value(right)?, tolerance))
}

/// Parse two texts in their respective formats and compare the results.
///
/// # Errors
///
/// Returns [`InteropError::Parse`] if either text is malformed, and
/// [`InteropError::InvalidSystem`] if either parses into an invalid system.
pub fn diff_text(
    left: &str,
    left_format: Format,
    right: &str,
    right_format: Format,
) -> InteropResult<Vec<Difference>> {
    diff_text_with_tolerance(left, left_format, right, right_format, DEFAULT_TOLERANCE)
}

/// Parse two texts in their respective formats and compare the results,
/// treating floating-point fields as equal within `tolerance`.
///
/// # Errors
///
/// Returns [`InteropError::Parse`] if either text is malformed, and
/// [`InteropError::InvalidSystem`] if either parses into an invalid system.
pub fn diff_text_with_tolerance(
    left: &str,
    left_format: Format,
    right: &str,
    right_format: Format,
    tolerance: f64,
) -> InteropResult<Vec<Difference>> {
    let a = crate::from_text(left, left_format)?;
    let b = crate::from_text(right, right_format)?;
    diff_systems_with_tolerance(&a, &b, tolerance)
}

/// Round-trip `text` through `format` and report what changed.
///
/// This is the format-migration check: parse, write in the same format, parse
/// the written output again, and compare the two models. An empty result means
/// the format is lossless for this case.
///
/// # Errors
///
/// Returns [`InteropError::Parse`] if the text is malformed or cannot be
/// written back out, and [`InteropError::InvalidSystem`] if it parses into an
/// invalid system.
pub fn round_trip(text: &str, format: Format) -> InteropResult<Vec<Difference>> {
    let original = crate::from_text(text, format)?;
    let written = crate::to_text(&original, format)?;
    let reparsed = crate::from_text(&written, format)?;
    diff_systems(&original, &reparsed)
}

/// Compare two JSON documents field by field.
///
/// Exposed so callers that already hold `serde_json::Value` data (the WASM and
/// Python bindings, for example) can reuse the same comparison rules.
#[must_use]
pub fn diff_values(left: &Value, right: &Value, tolerance: f64) -> Vec<Difference> {
    let mut out = Vec::new();
    walk("", left, right, tolerance, &mut out);
    out
}

/// Serialize a system for structural comparison.
fn to_value(system: &EnergySystem) -> InteropResult<Value> {
    serde_json::to_value(system).map_err(|e| InteropError::parse("json", e.to_string()))
}

/// Two numbers within `tolerance`, scaled by their own magnitude.
///
/// Bit-exact equality needs no special case: a difference of zero satisfies
/// the tolerance for any scale.
fn numbers_equal(a: f64, b: f64, tolerance: f64) -> bool {
    if !a.is_finite() || !b.is_finite() {
        // The JSON model cannot hold a non-finite number, so this only
        // happens for caller-supplied values; compare those bitwise.
        return a.to_bits() == b.to_bits();
    }
    let scale = a.abs().max(b.abs()).max(1.0);
    (a - b).abs() <= tolerance.abs() * scale
}

fn equal(left: &Value, right: &Value, tolerance: f64) -> bool {
    match (left.as_f64(), right.as_f64()) {
        (Some(a), Some(b)) => numbers_equal(a, b, tolerance),
        _ => left == right,
    }
}

fn child_path(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        key.to_string()
    } else {
        format!("{parent}.{key}")
    }
}

/// Maximum length of a rendered value before it is clipped.
const RENDER_LIMIT: usize = 72;

/// Render a value for a one-line human-readable report.
fn render(value: &Value) -> String {
    let text = match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("\"{s}\""),
        other => other.to_string(),
    };
    // Long composites (a whole record, a cost curve) stay readable when
    // clipped, and a diff report is scanned by eye rather than parsed.
    if text.chars().count() <= RENDER_LIMIT {
        text
    } else {
        let head: String = text.chars().take(RENDER_LIMIT).collect();
        format!("{head}...")
    }
}

/// Rendered stand-in for a path that is absent from one side.
const ABSENT: &str = "<absent>";

/// The `id` of a record, when the value is a JSON object that carries one.
fn record_id(value: &Value) -> Option<String> {
    value.as_object()?.get("id").map(render)
}

/// Index an array of records by `id`, or `None` when it cannot be keyed
/// (not objects, missing `id`, or duplicate ids). An empty array is keyed as
/// an empty map so that clearing a record list is reported by `id` rather
/// than by position.
fn keyed_by_id(items: &[Value]) -> Option<BTreeMap<String, &Value>> {
    let mut map = BTreeMap::new();
    for item in items {
        if map.insert(record_id(item)?, item).is_some() {
            return None;
        }
    }
    Some(map)
}

fn walk(path: &str, left: &Value, right: &Value, tolerance: f64, out: &mut Vec<Difference>) {
    if equal(left, right, tolerance) {
        return;
    }
    match (left, right) {
        (Value::Object(l), Value::Object(r)) => {
            let keys: BTreeSet<&String> = l.keys().chain(r.keys()).collect();
            for key in keys {
                let child = child_path(path, key);
                match (l.get(key), r.get(key)) {
                    (Some(lv), Some(rv)) => walk(&child, lv, rv, tolerance, out),
                    (Some(lv), None) => out.push(removed(child, lv)),
                    (None, Some(rv)) => out.push(added(child, rv)),
                    (None, None) => unreachable!("key is present in at least one side"),
                }
            }
        }
        (Value::Array(l), Value::Array(r)) => {
            if let (Some(lm), Some(rm)) = (keyed_by_id(l), keyed_by_id(r)) {
                let ids: BTreeSet<&String> = lm.keys().chain(rm.keys()).collect();
                for id in ids {
                    let child = format!("{path}[id={id}]");
                    match (lm.get(id), rm.get(id)) {
                        (Some(lv), Some(rv)) => walk(&child, lv, rv, tolerance, out),
                        (Some(lv), None) => out.push(removed(child, lv)),
                        (None, Some(rv)) => out.push(added(child, rv)),
                        (None, None) => unreachable!("id comes from one of the two sides"),
                    }
                }
            } else {
                walk_positional(path, l, r, tolerance, out);
            }
        }
        _ => out.push(Difference {
            kind: DifferenceKind::Changed,
            path: if path.is_empty() { "<root>" } else { path }.to_string(),
            left: render(left),
            right: render(right),
        }),
    }
}

fn walk_positional(
    path: &str,
    left: &[Value],
    right: &[Value],
    tolerance: f64,
    out: &mut Vec<Difference>,
) {
    let common = left.len().min(right.len());
    for (index, (lv, rv)) in left.iter().zip(right.iter()).enumerate() {
        walk(&format!("{path}[{index}]"), lv, rv, tolerance, out);
    }
    for (index, lv) in left.iter().enumerate().skip(common) {
        out.push(removed(format!("{path}[{index}]"), lv));
    }
    for (index, rv) in right.iter().enumerate().skip(common) {
        out.push(added(format!("{path}[{index}]"), rv));
    }
}

fn added(path: String, value: &Value) -> Difference {
    Difference {
        kind: DifferenceKind::Added,
        path,
        left: ABSENT.to_string(),
        right: render(value),
    }
}

fn removed(path: String, value: &Value) -> Difference {
    Difference {
        kind: DifferenceKind::Removed,
        path,
        left: render(value),
        right: ABSENT.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_nrg_core::{Branch, Bus, BusType, EnergySystem, Generator, GeneratorType};

    /// A two-bus case whose model lives entirely in the fields the text
    /// exchange formats carry (bus schedules, branch impedance, generator
    /// limits), so a round trip is expected to be lossless.
    fn two_bus() -> EnergySystem {
        let mut sys = EnergySystem::new("case", "Case", 100.0, 60.0);
        sys.add_bus(Bus::new(1, "B1", BusType::Slack).with_voltage_pu(1.06, 0.0))
            .unwrap();
        sys.add_bus(Bus::new(2, "B2", BusType::Pq).with_load(10.0, 5.0))
            .unwrap();
        sys.add_branch(Branch::new(1, "L1", 1, 2, 0.01, 0.1))
            .unwrap();
        sys.add_generator(Generator::new(1, "G1", GeneratorType::Thermal, 100.0, 10.0).at_bus(1))
            .unwrap();
        sys
    }

    #[test]
    fn identical_systems_have_no_differences() {
        let sys = two_bus();
        assert!(diff_systems(&sys, &sys).unwrap().is_empty());
    }

    #[test]
    fn a_changed_scalar_is_reported_with_its_path() {
        let left = two_bus();
        let mut right = two_bus();
        right.buses[1].load_mw = 12.5;
        let differences = diff_systems(&left, &right).unwrap();
        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].kind(), DifferenceKind::Changed);
        assert_eq!(differences[0].path(), "buses[id=2].load_mw");
        assert_eq!(differences[0].left(), "10.0");
        assert_eq!(differences[0].right(), "12.5");
        assert_eq!(
            differences[0].to_string(),
            "buses[id=2].load_mw: 10.0 -> 12.5"
        );
    }

    #[test]
    fn an_added_and_a_removed_record_are_both_reported() {
        let left = two_bus();
        let mut right = two_bus();
        right.buses.push(Bus::new(3, "B3", BusType::Pq));
        right.generators.clear();
        let differences = diff_systems(&left, &right).unwrap();
        let paths: Vec<&str> = differences.iter().map(Difference::path).collect();
        assert!(paths.contains(&"buses[id=3]"), "{paths:?}");
        assert!(paths.contains(&"generators[id=1]"), "{paths:?}");
        let removed = differences
            .iter()
            .find(|d| d.path() == "generators[id=1]")
            .expect("removed record");
        assert_eq!(removed.kind(), DifferenceKind::Removed);
        assert_eq!(removed.right(), ABSENT);
    }

    #[test]
    fn record_order_does_not_matter() {
        let left = two_bus();
        let mut right = two_bus();
        right.buses.reverse();
        assert!(diff_systems(&left, &right).unwrap().is_empty());
    }

    #[test]
    fn a_different_name_is_a_change() {
        let left = two_bus();
        let mut right = two_bus();
        right.buses[0].name = "Slack".to_string();
        let differences = diff_systems(&left, &right).unwrap();
        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].path(), "buses[id=1].name");
        assert_eq!(differences[0].left(), "\"B1\"");
    }

    #[test]
    fn float_noise_below_the_tolerance_is_ignored() {
        let left = two_bus();
        let mut right = two_bus();
        right.buses[1].load_mw += 1e-15;
        assert!(diff_systems(&left, &right).unwrap().is_empty());
    }

    #[test]
    fn a_custom_tolerance_absorbs_larger_noise() {
        let left = two_bus();
        let mut right = two_bus();
        right.buses[1].load_mw = 10.001;
        assert_eq!(diff_systems(&left, &right).unwrap().len(), 1);
        assert!(diff_systems_with_tolerance(&left, &right, 1e-2)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn a_changed_top_level_scalar_is_reported() {
        let left = two_bus();
        let mut right = two_bus();
        right.base_mva = 200.0;
        let differences = diff_systems(&left, &right).unwrap();
        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].path(), "base_mva");
    }

    #[test]
    fn a_json_case_matches_the_matpower_export_of_the_same_case() {
        let sys = two_bus();
        let json = sys.to_json_pretty().unwrap();
        let matpower = crate::to_text(&sys, Format::Matpower).unwrap();
        let differences = diff_text(&json, Format::Json, &matpower, Format::Matpower).unwrap();
        // The model is identical; only fields MATPOWER does not carry differ.
        let paths: Vec<&str> = differences.iter().map(Difference::path).collect();
        for path in &paths {
            assert!(
                path.contains("name") || path.contains("metadata"),
                "unexpected difference at {path}: {paths:?}"
            );
        }
    }

    #[test]
    fn a_json_round_trip_is_lossless() {
        let text = two_bus().to_json_pretty().unwrap();
        assert!(round_trip(&text, Format::Json).unwrap().is_empty());
    }

    #[test]
    fn a_matpower_round_trip_preserves_the_model() {
        let text = crate::to_text(&two_bus(), Format::Matpower).unwrap();
        let differences = round_trip(&text, Format::Matpower).unwrap();
        let paths: Vec<&str> = differences.iter().map(Difference::path).collect();
        for path in &paths {
            assert!(
                path.contains("name") || path.contains("metadata"),
                "unexpected difference at {path}: {paths:?}"
            );
        }
    }

    #[test]
    fn a_yaml_case_round_trips_losslessly() {
        let text = crate::to_text(&two_bus(), Format::Yaml).unwrap();
        assert!(round_trip(&text, Format::Yaml).unwrap().is_empty());
    }

    #[test]
    fn a_removed_top_level_field_is_reported() {
        let left = serde_json::json!({"id": "case", "name": "Case"});
        let right = serde_json::json!({"id": "case"});
        let differences = diff_values(&left, &right, DEFAULT_TOLERANCE);
        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].kind(), DifferenceKind::Removed);
        assert_eq!(differences[0].path(), "name");
        assert_eq!(differences[0].right(), ABSENT);
    }

    #[test]
    fn arrays_without_ids_compare_positionally() {
        let left = serde_json::json!({"cost_curve": [1.0, 2.0, 3.0]});
        let right = serde_json::json!({"cost_curve": [1.0, 2.5, 3.0]});
        let differences = diff_values(&left, &right, DEFAULT_TOLERANCE);
        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].path(), "cost_curve[1]");
    }

    #[test]
    fn different_length_arrays_report_the_tail() {
        let left = serde_json::json!({"points": [1.0, 2.0]});
        let right = serde_json::json!({"points": [1.0, 2.0, 3.0]});
        let differences = diff_values(&left, &right, DEFAULT_TOLERANCE);
        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].kind(), DifferenceKind::Added);
        assert_eq!(differences[0].path(), "points[2]");
    }

    #[test]
    fn long_composites_are_clipped() {
        let points: Vec<f64> = (1..=40).map(f64::from).collect();
        let rendered = render(&serde_json::json!(points));
        assert!(rendered.ends_with("..."), "{rendered}");
        assert!(rendered.chars().count() <= 75);
    }

    #[test]
    fn malformed_input_is_a_parse_error() {
        assert!(diff_text("{", Format::Json, "{}", Format::Json).is_err());
        assert!(round_trip("{", Format::Json).is_err());
    }
}
