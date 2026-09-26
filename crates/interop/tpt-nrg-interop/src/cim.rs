//! CIM / IEC 61970 import and export.
//!
//! CIM exchange files are RDF/XML documents. This module implements the
//! subset of the standard that carries a steady-state network model:
//!
//! - `BaseVoltage.nominalVoltage` — base kV of a voltage level.
//! - `VoltageLevel` / `Substation` — containers.
//! - `BusbarSection`, `ConnectivityNode`, `Terminal` — the node graph.
//! - `ACLineSegment` (with `ACLineSegment.r`, `.x`, `.b`, `.shortCircuitEndTemperature`,
//!   and `CurrentLimit`), `PowerTransformer` (via `TransformerEnd`).
//! - `EnergyConsumer` (`EnergyConsumer.p`, `.q`), `RotatingMachine`
//!   (`RotatingMachine.p`, `.q`, `.minOperatingP`, `.maxOperatingP`).
//! - `EquivalentInjection` — the slack equivalent.
//!
//! ## Modelling decisions
//!
//! A full CIM profile has no single unambiguous mapping onto the TPT Energy
//! data model, so this converter uses the conventional one and documents it:
//!
//! | TPT Energy   | CIM                                                  |
//! |--------------|------------------------------------------------------|
//! | `Bus`        | a `ConnectivityNode` reachable from `BusbarSection`   |
//! | `Branch`     | an in-service `ACLineSegment`; `PowerTransformer` is imported as a branch with a unity tap unless a `TransformerEnd.ratio` is present |
//! | `Generator`  | a `RotatingMachine` or `EquivalentInjection`         |
//! | bus load     | summed `EnergyConsumer.p` / `.q` attached to the node |
//!
//! Documents are read by *local* name, so any namespace prefix
//! (`cim:`, `md:`, or none) is accepted.

use std::collections::HashMap;
use std::fmt::Write as _;

use tpt_nrg_core::{Branch, Bus, BusType, EnergySystem, Generator, GeneratorType, Load};

use crate::error::{InteropError, InteropResult};

/// Degrees-to-radians conversion factor.
const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;

/// A parsed XML element: local tag name, attributes, children, and text.
#[derive(Debug, Clone, Default)]
pub struct XmlElement {
    /// Tag name with any namespace prefix removed.
    pub name: String,
    /// Attributes with prefixes removed from their names.
    pub attributes: HashMap<String, String>,
    /// Direct child elements.
    pub children: Vec<XmlElement>,
    /// Concatenated text content of this element.
    pub text: String,
}

impl XmlElement {
    /// Return the first direct child with the given local name.
    fn child(&self, name: &str) -> Option<&XmlElement> {
        self.children.iter().find(|c| c.name == name)
    }

    /// Return the first direct child with any of the given local names.
    fn child_any(&self, names: &[&str]) -> Option<&XmlElement> {
        self.children
            .iter()
            .find(|c| names.iter().any(|n| *n == c.name))
    }

    /// Return the trimmed text of the first child with the given name.
    fn child_text(&self, name: &str) -> Option<String> {
        self.child(name).map(|c| c.text.trim().to_string())
    }

    /// Parse the trimmed text of the first child as `f64`.
    fn child_num(&self, name: &str) -> Option<f64> {
        self.child_text(name).and_then(|t| t.parse::<f64>().ok())
    }

    /// Return the trimmed text of the first child with any of the given names,
    /// parsed as `f64`.
    fn child_num_any(&self, names: &[&str]) -> Option<f64> {
        self.child_any(names)
            .and_then(|c| c.text.trim().parse::<f64>().ok())
    }

    /// Return the `rdf:resource` / `rdf:about` reference of a child.
    fn child_ref(&self, name: &str) -> Option<String> {
        let child = self.child(name)?;
        child
            .attributes
            .get("resource")
            .or_else(|| child.attributes.get("about"))
            .cloned()
    }

    /// Return the `rdf:about` identifier of this element, if any.
    fn about(&self) -> Option<String> {
        self.attributes.get("about").cloned()
    }

    /// Return all direct children with the given local name.
    fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a XmlElement> {
        self.children.iter().filter(move |c| c.name == name)
    }

    /// Resolve an `rdf:about` / `rdf:resource` reference to a local name.
    ///
    /// CIM references are URIs; only the fragment or the last path segment
    /// carries identity.
    fn local_id(reference: &str) -> &str {
        reference
            .rsplit_once('#')
            .map_or(reference, |(_, frag)| frag)
            .rsplit_once('/')
            .map_or(reference, |(_, seg)| seg)
    }
}

/// Strip a namespace prefix from a qualified name (`cim:BaseVoltage` →
/// `BaseVoltage`).
fn local_name(qname: &str) -> String {
    qname
        .rsplit_once(':')
        .map_or(qname, |(_, local)| local)
        .to_string()
}

/// Decode the five XML entities that appear in RDF/XML payloads.
fn decode_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Split an XML tag's attribute list into `(local name, value)` pairs.
///
/// Handles quoted values, self-closing tags, and namespace declarations,
/// which are dropped.
fn parse_attributes(body: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        while i < chars.len() && (chars[i].is_whitespace() || chars[i] == '/') {
            i += 1;
        }
        let start = i;
        while i < chars.len() && chars[i] != '=' && !chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        let name: String = chars[start..i].iter().collect();
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() || chars[i] != '=' {
            continue;
        }
        i += 1;
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        let quote = chars[i];
        if quote != '"' && quote != '\'' {
            continue;
        }
        i += 1;
        let vstart = i;
        while i < chars.len() && chars[i] != quote {
            i += 1;
        }
        let value: String = chars[vstart..i.min(chars.len())].iter().collect();
        i += 1;
        if !name.is_empty() && name != "xmlns" && !name.starts_with("xmlns:") {
            out.insert(local_name(&name), decode_entities(&value));
        }
    }
    out
}

/// Find `marker` in `chars` at or after `from`, returning its start index.
fn find_marker(chars: &[char], from: usize, marker: &str) -> Option<usize> {
    let m: Vec<char> = marker.chars().collect();
    let last = chars.len().checked_sub(m.len())?;
    (from..=last).find(|&i| chars[i..i + m.len()] == m[..])
}

/// Parse an RDF/XML document into a tree of [`XmlElement`].
///
/// This is a deliberately small, non-validating parser: it tracks element
/// nesting, skips comments, CDATA sections, and processing instructions, and
/// rejects unbalanced tags with [`InteropError::Parse`].
///
/// # Errors
///
/// Returns [`InteropError::Parse`] if a tag is unterminated, a closing tag
/// does not match its opener, or the document ends with unclosed elements.
pub fn parse_xml(text: &str) -> InteropResult<XmlElement> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut stack: Vec<XmlElement> = vec![XmlElement {
        name: "#document".to_string(),
        ..XmlElement::default()
    }];

    while i < chars.len() {
        if chars[i] != '<' {
            let start = i;
            while i < chars.len() && chars[i] != '<' {
                i += 1;
            }
            let chunk: String = chars[start..i].iter().collect();
            if let Some(parent) = stack.last_mut() {
                parent.text.push_str(&decode_entities(&chunk));
            }
            continue;
        }
        if chars[i..].starts_with(&['<', '!', '-', '-']) {
            let end = find_marker(&chars, i, "-->")
                .ok_or_else(|| InteropError::parse("cim", "unterminated comment"))?;
            i = end + 3;
            continue;
        }
        if chars[i..].starts_with(&['<', '?']) {
            let end = find_marker(&chars, i, "?>").ok_or_else(|| {
                InteropError::parse("cim", "unterminated processing instruction")
            })?;
            i = end + 2;
            continue;
        }
        if chars.get(i + 1) == Some(&'/') {
            i = close_element(&chars, i, &mut stack)?;
            continue;
        }
        i = open_element(&chars, i, &mut stack)?;
    }

    if stack.len() > 1 {
        let unclosed = stack.last().map_or("?", |e| e.name.as_str());
        return Err(InteropError::parse(
            "cim",
            format!("unclosed element <{unclosed}> at end of document"),
        ));
    }
    stack
        .pop()
        .ok_or_else(|| InteropError::parse("cim", "empty document"))
}

/// Handle a closing tag at `i`, returning the index just past it.
fn close_element(
    chars: &[char],
    i: usize,
    stack: &mut Vec<XmlElement>,
) -> InteropResult<usize> {
    let end = find_marker(chars, i, ">")
        .ok_or_else(|| InteropError::parse("cim", "unterminated closing tag"))?;
    let raw: String = chars[i + 2..end].iter().collect();
    let name = local_name(raw.trim());
    let closed = stack
        .pop()
        .ok_or_else(|| InteropError::parse("cim", format!("closing tag </{name}> with no opener")))?;
    if closed.name != name {
        return Err(InteropError::parse(
            "cim",
            format!("closing tag </{name}> does not match <{}>", closed.name),
        ));
    }
    if let Some(parent) = stack.last_mut() {
        parent.children.push(closed);
    }
    Ok(end + 1)
}

/// Handle an opening (possibly self-closing) tag at `i`, returning the index
/// just past it.
fn open_element(chars: &[char], i: usize, stack: &mut Vec<XmlElement>) -> InteropResult<usize> {
    let end = find_marker(chars, i, ">")
        .ok_or_else(|| InteropError::parse("cim", "unterminated opening tag"))?;
    let body: String = chars[i + 1..end].iter().collect();
    let trimmed = body.trim_end();
    let self_closing = trimmed.ends_with('/');
    let trimmed = trimmed.trim_end_matches('/');
    let name = local_name(trimmed.split_whitespace().next().unwrap_or(""));
    if name.is_empty() {
        return Err(InteropError::parse("cim", "empty tag name"));
    }
    let element = XmlElement {
        name,
        attributes: parse_attributes(trimmed),
        children: Vec::new(),
        text: String::new(),
    };
    if self_closing {
        if let Some(parent) = stack.last_mut() {
            parent.children.push(element);
        }
    } else {
        stack.push(element);
    }
    Ok(end + 1)
}

/// An index over every identified object in a CIM document.
#[derive(Debug, Default)]
struct CimIndex {
    /// Objects by their `rdf:about` local identifier.
    by_id: HashMap<String, XmlElement>,
}

impl CimIndex {
    /// Build the index from a parsed document.
    fn build(root: &XmlElement) -> Self {
        let mut by_id = HashMap::new();
        // An RDF/XML document nests all described objects under <RDF>, but
        // some exporters also emit them as siblings of <RDF>; index both.
        let containers: Vec<&XmlElement> = root
            .children
            .iter()
            .filter(|c| c.name == "RDF")
            .chain(root.children.iter().filter(|c| c.name != "RDF"))
            .collect();
        for container in containers {
            index_children(container, &mut by_id);
        }
        Self { by_id }
    }

    /// Look up an object by a possibly-qualified reference.
    fn get(&self, reference: &str) -> Option<&XmlElement> {
        self.by_id.get(XmlElement::local_id(reference))
    }

    /// Follow a chain of single-reference child elements.
    fn follow<'a>(&'a self, start: &'a XmlElement, path: &[&str]) -> Option<&'a XmlElement> {
        let mut current = start;
        for step in path {
            let reference = current.child_ref(step)?;
            current = self.get(&reference)?;
        }
        Some(current)
    }

    /// All indexed objects of a given class name.
    fn of_class<'a>(&'a self, class: &'a str) -> impl Iterator<Item = &'a XmlElement> {
        self.by_id.values().filter(move |e| e.name == class)
    }
}

/// Recursively index an element and its children by `rdf:about`.
fn index_children(element: &XmlElement, out: &mut HashMap<String, XmlElement>) {
    if let Some(id) = element.about() {
        out.insert(XmlElement::local_id(&id).to_string(), element.clone());
    }
    for child in &element.children {
        index_children(child, out);
    }
}

/// Equipment classes that become a [`Generator`].
const MACHINE_CLASSES: [&str; 2] = ["RotatingMachine", "EquivalentInjection"];

/// Parse a CIM / IEC 61970 RDF/XML document into an [`EnergySystem`].
///
/// See the [module documentation](self) for the mapping this converter uses.
///
/// # Errors
///
/// Returns [`InteropError::Parse`] if the document is not well-formed XML,
/// [`InteropError::Missing`] if it contains no `ConnectivityNode`, and
/// [`InteropError::InvalidSystem`] if the result fails structural validation.
pub fn from_cim(text: &str) -> InteropResult<EnergySystem> {
    let root = parse_xml(text)?;
    let index = CimIndex::build(&root);
    let nodes: Vec<&XmlElement> = index.of_class("ConnectivityNode").collect();
    if nodes.is_empty() {
        return Err(InteropError::missing(
            "cim",
            "at least one ConnectivityNode element",
        ));
    }

    let mut system = read_model(&index)?;
    system.metadata.insert("source".into(), serde_json::json!("CIM/IEC 61970"));
    system.validate().map_err(InteropError::InvalidSystem)?;
    Ok(system)
}

/// Read buses, branches, generators, and loads out of a CIM index.
fn read_model(index: &CimIndex) -> InteropResult<EnergySystem> {
    let nodes: Vec<&XmlElement> = index.of_class("ConnectivityNode").collect();
    let model = index.of_class("Model").next();
    let base_mva = model
        .and_then(|m| m.child_num("Model.baseMVA"))
        .filter(|v| *v > 0.0)
        .unwrap_or(100.0);
    let frequency_hz = model
        .and_then(|m| m.child_num("Model.frequency"))
        .filter(|v| *v > 0.0)
        .unwrap_or(60.0);
    let mut sys = EnergySystem::new("cim", "CIM model", base_mva, frequency_hz);

    for node in &nodes {
        let bus = cim_bus_to_core(index, node)?;
        sys.add_bus(bus).map_err(InteropError::InvalidSystem)?;
    }

    add_cim_branches(index, &mut sys)?;
    add_cim_machines(index, &mut sys)?;
    add_cim_loads(index, &mut sys)?;
    Ok(sys)
}

/// Convert a `ConnectivityNode` to a [`Bus`], with its loads and machine flags.
fn cim_bus_to_core(index: &CimIndex, node: &XmlElement) -> InteropResult<Bus> {
    let id = node.about().ok_or_else(|| {
        InteropError::bad_value("cim", "ConnectivityNode", "missing rdf:about identifier")
    })?;
    let local = XmlElement::local_id(&id);
    let bus_id = local.parse::<usize>().map_err(|_| {
        InteropError::bad_value(
            "cim",
            "ConnectivityNode rdf:about",
            format!("node id `{local}` is not an integer bus number"),
        )
    })?;
    let name = node
        .child_text("IdentifiedObject.name")
        .or_else(|| node.child_text("name"))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("Bus {bus_id}"));
    let (v, angle_deg) = topological_voltage(index, node).unwrap_or((1.0, 0.0));
    let base_kv = node_base_kv(index, node).unwrap_or(100.0);

    let mut has_machine = false;
    let mut has_slack = false;
    for class in equipment_classes_at(index, node) {
        if MACHINE_CLASSES.contains(&class.as_str()) {
            has_machine = true;
            has_slack |= class == "EquivalentInjection";
        }
    }
    let (load_mw, load_mvar) = consumer_totals(index, node);
    let bus_type = if has_slack {
        BusType::Slack
    } else if has_machine {
        BusType::Pv
    } else {
        BusType::Pq
    };

    Ok(
        Bus::new(bus_id, name, bus_type)
            .with_voltage_pu(v, angle_deg * DEG_TO_RAD)
            .with_base_kv(base_kv)
            .with_load(load_mw, load_mvar),
    )
}

/// Nominal voltage in kV of the voltage level containing a node.
fn node_base_kv(index: &CimIndex, node: &XmlElement) -> Option<f64> {
    index
        .follow(node, &["Container", "BaseVoltage"])?
        .child_num("BaseVoltage.nominalVoltage")
        .filter(|v| *v > 0.0)
}

/// Voltage magnitude and angle (degrees) of a node, from `TopologicalNode`.
///
/// CIM keeps steady-state voltages on the topological node rather than the
/// connectivity node, so the lookup is by `TopologicalNode.ConnectivityNode`
/// or by a `TopologicalNode.equipment` reference.
fn topological_voltage(index: &CimIndex, node: &XmlElement) -> Option<(f64, f64)> {
    let id = node.about()?;
    let local = XmlElement::local_id(&id);
    index.of_class("TopologicalNode").find_map(|topo| {
        let direct = topo
            .child_ref("TopologicalNode.ConnectivityNode")
            .is_some_and(|r| XmlElement::local_id(&r) == local);
        let via_equipment = topo
            .children_named("TopologicalNode.equipment")
            .filter_map(|c| c.attributes.get("resource"))
            .any(|r| XmlElement::local_id(r) == local);
        if direct || via_equipment {
            Some((
                topo.child_num("SvVoltage.v").unwrap_or(1.0),
                topo.child_num("SvVoltage.angle").unwrap_or(0.0),
            ))
        } else {
            None
        }
    })
}

/// Conducting equipment attached to a node, reached through its terminals.
fn equipment_at<'a>(index: &'a CimIndex, node: &XmlElement) -> Vec<&'a XmlElement> {
    let mut out = Vec::new();
    for terminal_ref in node.children_named("ConnectivityNode.Terminals") {
        let Some(reference) = terminal_ref.attributes.get("resource") else {
            continue;
        };
        let Some(terminal) = index.get(reference) else {
            continue;
        };
        if let Some(equipment) = terminal.child_ref("Terminal.ConductingEquipment") {
            if let Some(element) = index.get(&equipment) {
                out.push(element);
            }
        }
    }
    out
}

/// Class names of the equipment attached to a node.
fn equipment_classes_at(index: &CimIndex, node: &XmlElement) -> Vec<String> {
    equipment_at(index, node)
        .iter()
        .map(|e| e.name.clone())
        .collect()
}

/// Summed `p` and `q` of the `EnergyConsumer`s attached to a node.
fn consumer_totals(index: &CimIndex, node: &XmlElement) -> (f64, f64) {
    equipment_at(index, node)
        .iter()
        .filter(|e| e.name == "EnergyConsumer")
        .fold((0.0, 0.0), |(p, q), c| {
            (
                p + c.child_num_any(&["EnergyConsumer.p"]).unwrap_or(0.0),
                q + c.child_num_any(&["EnergyConsumer.q"]).unwrap_or(0.0),
            )
        })
}

/// Bus ids a piece of equipment is attached to, via its terminals.
fn equipment_buses(index: &CimIndex, equipment: &XmlElement) -> Vec<usize> {
    let Some(id) = equipment.about() else {
        return Vec::new();
    };
    let local = XmlElement::local_id(&id);
    let mut buses = Vec::new();
    for terminal in index.of_class("Terminal") {
        if terminal.child_ref("Terminal.ConductingEquipment")
            .is_some_and(|r| XmlElement::local_id(&r) == local)
        {
            if let Some(node) = terminal.child_ref("Terminal.ConnectivityNode") {
                let node_local = XmlElement::local_id(&node);
                if let Some(v) = equipment_buses_from_id(index, node_local) {
                    buses.push(v);
                }
            }
        }
    }
    buses
}

/// Resolve a `ConnectivityNode` reference to its integer bus number.
fn equipment_buses_from_id(index: &CimIndex, node_local: &str) -> Option<usize> {
    index
        .of_class("ConnectivityNode")
        .find(|n| n.about().is_some_and(|a| XmlElement::local_id(&a) == node_local))
        .and_then(|n| {
            n.about()
                .and_then(|a| XmlElement::local_id(&a).parse::<usize>().ok())
        })
}

/// Add every two-terminal in-service `ACLineSegment` as a [`Branch`].
fn add_cim_branches(index: &CimIndex, sys: &mut EnergySystem) -> InteropResult<()> {
    let mut segments = sorted_by_id(index.of_class("ACLineSegment"));
    for (i, segment) in segments.drain(..).enumerate() {
        let ends = equipment_buses(index, segment);
        let [from_bus, to_bus] = ends.as_slice() else {
            // Anything other than exactly two ends is not a two-terminal
            // branch; skip it rather than guess an end.
            continue;
        };
        let in_service = segment
            .child_num("ACLineSegment.inService")
            .is_none_or(|v| v != 0.0);
        let name = segment
            .child_text("IdentifiedObject.name")
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("{from_bus}-{to_bus}"));
        let mut branch = Branch::new(
            i + 1,
            name,
            *from_bus,
            *to_bus,
            segment.child_num("ACLineSegment.r").unwrap_or(0.0),
            segment.child_num("ACLineSegment.x").unwrap_or(0.01),
        )
        .with_susceptance(segment.child_num("ACLineSegment.b").unwrap_or(0.0))
        .with_rating(cim_rating_mva(index, segment))
        .with_in_service(in_service);
        if let Some(tap) = segment.child_num("ACLineSegment.ratio") {
            branch = branch.with_tap(tap, 0.0);
        }
        sys.add_branch(branch)
            .map_err(InteropError::InvalidSystem)?;
    }
    Ok(())
}

/// Collect identified objects in a deterministic (id-sorted) order.
///
/// CIM documents are unordered; sorting makes import reproducible so that
/// generated ids and exports are stable across runs.
fn sorted_by_id<'a, I>(items: I) -> Vec<&'a XmlElement>
where
    I: Iterator<Item = &'a XmlElement>,
{
    let mut out: Vec<&XmlElement> = items.collect();
    out.sort_by_key(|e| e.about().as_deref().map_or_else(String::new, ToString::to_string));
    out
}

/// Thermal rating in MVA of a line, from its current limit and the base kV of
/// a node it connects.
fn cim_rating_mva(index: &CimIndex, segment: &XmlElement) -> f64 {
    let current_limit = segment
        .child_num("ACLineSegment.CurrentLimit")
        .or_else(|| {
            segment
                .children_named("CurrentLimit.value")
                .find_map(|c| c.text.trim().parse::<f64>().ok())
        })
        .unwrap_or(0.0);
    if current_limit <= 0.0 {
        return 100.0;
    }
    let base_kv = equipment_buses(index, segment)
        .iter()
        .find_map(|bus| bus_base_kv(index, *bus))
        .unwrap_or(100.0);
    (current_limit * base_kv / 1000.0).max(0.1)
}

/// Nominal voltage in kV of a bus, looked up by integer bus number.
fn bus_base_kv(index: &CimIndex, bus: usize) -> Option<f64> {
    index
        .of_class("ConnectivityNode")
        .find(|n| {
            n.about()
                .and_then(|a| XmlElement::local_id(&a).parse::<usize>().ok())
                == Some(bus)
        })
        .and_then(|n| node_base_kv(index, n))
}

/// Add every `RotatingMachine` and `EquivalentInjection` as a [`Generator`].
///
/// CIM has no fuel-type attribute, so machines are typed
/// [`GeneratorType::Thermal`] unless the object name identifies a renewable,
/// which exporters commonly do.
fn add_cim_machines(index: &CimIndex, sys: &mut EnergySystem) -> InteropResult<()> {
    let machines = sorted_by_id(
        index
            .of_class("RotatingMachine")
            .chain(index.of_class("EquivalentInjection")),
    );
    for (i, machine) in machines.into_iter().enumerate() {
        let Some(bus_id) = equipment_buses(index, machine).first().copied() else {
            continue;
        };
        let p = machine
            .child_num_any(&["RotatingMachine.p", "EquivalentInjection.p"])
            .unwrap_or(0.0);
        let q = machine
            .child_num_any(&["RotatingMachine.q", "EquivalentInjection.q"])
            .unwrap_or(0.0);
        let p_max = machine
            .child_num_any(&[
                "RotatingMachine.maxOperatingP",
                "EquivalentInjection.maxP",
            ])
            .unwrap_or_else(|| p.abs().max(1.0));
        let p_min = machine
            .child_num_any(&[
                "RotatingMachine.minOperatingP",
                "EquivalentInjection.minP",
            ])
            .unwrap_or(0.0);
        let name = machine
            .child_text("IdentifiedObject.name")
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("G{}", i + 1));
        let mut gen = Generator::new(
            i + 1,
            name.clone(),
            cim_generator_type(&name),
            p_max,
            p_min,
        )
        .at_bus(bus_id)
        .with_p_schedule(p)
        .with_reactive_limits(-q.abs(), q.abs());
        if let Some(v) = machine.child_num("RotatingMachine.regulatedControl.targetValue") {
            gen = gen.with_voltage_setpoint(v);
        }
        sys.add_generator(gen)
            .map_err(InteropError::InvalidSystem)?;
    }
    Ok(())
}

/// Infer a generator technology from a CIM object name.
fn cim_generator_type(name: &str) -> GeneratorType {
    let lower = name.to_ascii_lowercase();
    if lower.contains("wind") {
        GeneratorType::Wind
    } else if lower.contains("solar") || lower.contains("pv") {
        GeneratorType::Solar
    } else if lower.contains("hydro") {
        GeneratorType::Hydro
    } else if lower.contains("nuclear") {
        GeneratorType::Nuclear
    } else {
        GeneratorType::Thermal
    }
}

/// Add every `EnergyConsumer` as a [`Load`].
///
/// Bus loads are *not* re-imported as consumers: the importer has already
/// folded them into the bus, so re-adding them would double-count.
fn add_cim_loads(index: &CimIndex, sys: &mut EnergySystem) -> InteropResult<()> {
    let consumers = sorted_by_id(index.of_class("EnergyConsumer"));
    for (i, consumer) in consumers.into_iter().enumerate() {
        let Some(bus_id) = equipment_buses(index, consumer).first().copied() else {
            continue;
        };
        let name = consumer
            .child_text("IdentifiedObject.name")
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("L{}", i + 1));
        let mut load = Load::new(
            i + 1,
            name,
            bus_id,
            consumer.child_num("EnergyConsumer.p").unwrap_or(0.0),
            consumer.child_num("EnergyConsumer.q").unwrap_or(0.0),
        );
        load.in_service = true;
        sys.add_load(load).map_err(InteropError::InvalidSystem)?;
    }
    Ok(())
}

/// RDF/XML namespace declarations used by the exporter.
const CIM_NAMESPACES: [(&str, &str); 3] = [
    ("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
    ("cim", "http://iec.ch/TC57/2013/CIM-schema-cim16#"),
    ("md", "http://iec.ch/TC57/2013/CIM-schema-cim16#"),
];

/// Escape the five XML entities for element text.
fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Serialize an [`EnergySystem`] as a CIM / IEC 61970 RDF/XML document.
///
/// The emitted profile is exactly the one [`from_cim`] reads, so a document
/// round-trips:
///
/// - one `BaseVoltage` and one `VoltageLevel` per distinct base kV,
/// - one `ConnectivityNode` per bus, each holding a `BusbarSection` terminal
///   plus a terminal per attached machine and load,
/// - one `ACLineSegment` with two terminals per branch,
/// - one `RotatingMachine` (or `EquivalentInjection`, at the slack bus) per
///   generator, and one `EnergyConsumer` per bus with a non-zero load.
///
/// Object identifiers are `urn:tpt-energy:<Class>:<n>`, and node identifiers
/// are the bare bus number so that the importer recovers the original ids.
///
/// # Errors
///
/// This function cannot fail for any [`EnergySystem`] — every value is
/// formatted as a float and every name is XML-escaped. The `Result` is kept
/// so that future profile additions can fail without a breaking change.
#[allow(clippy::unnecessary_wraps)]
pub fn to_cim(system: &EnergySystem) -> InteropResult<String> {
    let mut out = String::with_capacity(8192);
    let _ = writeln!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    out.push_str("<rdf:RDF");
    for (prefix, uri) in CIM_NAMESPACES {
        let _ = write!(out, " xmlns:{prefix}=\"{uri}\"");
    }
    let _ = writeln!(out, ">");
    let _ = writeln!(out, "  <cim:Model rdf:about=\"urn:tpt-energy:Model\">");
    let _ = writeln!(out, "    <cim:Model.baseMVA>{}</cim:Model.baseMVA>", system.base_mva);
    let _ = writeln!(out, "    <cim:Model.frequency>{}</cim:Model.frequency>", system.frequency_hz);
    let _ = writeln!(out, "  </cim:Model>");

    for kv in distinct_base_kv(system) {
        let kv_id = format!("urn:tpt-energy:BaseVoltage:{kv}");
        let vl_id = format!("urn:tpt-energy:VoltageLevel:{kv}");
        let _ = writeln!(out, "  <cim:BaseVoltage rdf:about=\"{kv_id}\">");
        let _ = writeln!(out, "    <cim:BaseVoltage.nominalVoltage>{kv}</cim:BaseVoltage.nominalVoltage>");
        let _ = writeln!(out, "  </cim:BaseVoltage>");
        let _ = writeln!(out, "  <cim:VoltageLevel rdf:about=\"{vl_id}\">");
        let _ = writeln!(out, "    <cim:VoltageLevel.BaseVoltage rdf:resource=\"{kv_id}\"/>");
        let _ = writeln!(out, "  </cim:VoltageLevel>");
    }

    let mut ids = Ids::default();
    for b in &system.buses {
        write_cim_bus(&mut out, &mut ids, system, b);
    }
    for br in &system.branches {
        write_cim_line(&mut out, &mut ids, system, br);
    }
    for l in &system.loads {
        let id = ids.next_id("EnergyConsumer");
        let term = ids.next_id("Terminal");
        write_object_header(&mut out, "EnergyConsumer", &id, &format!("Load at bus {}", l.bus_id));
        let _ = writeln!(out, "    <cim:EnergyConsumer.p>{}</cim:EnergyConsumer.p>", l.p_mw);
        let _ = writeln!(out, "    <cim:EnergyConsumer.q>{}</cim:EnergyConsumer.q>", l.q_mvar);
        let _ = writeln!(out, "    <cim:EnergyConsumer.Terminals rdf:resource=\"{term}\"/>");
        let _ = writeln!(out, "  </cim:EnergyConsumer>");
        write_terminal(&mut out, &term, &id, l.bus_id, "EnergyConsumer");
    }

    let _ = writeln!(out, "</rdf:RDF>");
    Ok(out)
}

/// Monotonic object-id allocator for the exporter.
#[derive(Debug, Default)]
struct Ids {
    /// Next numeric suffix.
    next: usize,
}

impl Ids {
    /// Allocate the next identifier for a class.
    fn next_id(&mut self, class: &str) -> String {
        self.next += 1;
        format!("urn:tpt-energy:{class}:{n}", n = self.next)
    }
}

/// Distinct bus base voltages, so one `BaseVoltage` is emitted per level.
fn distinct_base_kv(system: &EnergySystem) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for b in &system.buses {
        let kv = format!("{}", b.base_kv);
        if !out.contains(&kv) {
            out.push(kv);
        }
    }
    out
}

/// The `ConnectivityNode` identifier of a bus: the bare bus number.
fn node_id(bus: usize) -> String {
    bus.to_string()
}

/// Open an object element with a name and identifier.
fn write_object_header(out: &mut String, class: &str, id: &str, name: &str) {
    let _ = writeln!(out, "  <cim:{class} rdf:about=\"{id}\">");
    let _ = writeln!(
        out,
        "    <cim:IdentifiedObject.name>{}</cim:IdentifiedObject.name>",
        xml_escape(name)
    );
}

/// Emit a `Terminal` binding a piece of equipment to a connectivity node.
fn write_terminal(out: &mut String, term_id: &str, equipment_id: &str, bus: usize, class: &str) {
    let _ = writeln!(out, "  <cim:Terminal rdf:about=\"{term_id}\">");
    let _ = writeln!(out, "    <cim:Terminal.ConductingEquipment rdf:resource=\"{equipment_id}\" rdf:about=\"{class}\"/>");
    let _ = writeln!(out, "    <cim:Terminal.ConnectivityNode rdf:resource=\"{}\"/>", node_id(bus));
    let _ = writeln!(out, "  </cim:Terminal>");
}

/// Emit the connectivity node, busbar, machines, and load for one bus.
fn write_cim_bus(out: &mut String, ids: &mut Ids, system: &EnergySystem, bus: &Bus) {
    let mut terminals: Vec<String> = Vec::new();

    let bb_id = ids.next_id("BusbarSection");
    let bb_term = ids.next_id("Terminal");
    write_object_header(out, "BusbarSection", &bb_id, &bus.name);
    let _ = writeln!(out, "    <cim:BusbarSection.Terminals rdf:resource=\"{bb_term}\"/>");
    let _ = writeln!(out, "  </cim:BusbarSection>");
    write_terminal(out, &bb_term, &bb_id, bus.id, "BusbarSection");
    terminals.push(bb_term);

    let machines: Vec<&Generator> = system
        .generators
        .iter()
        .filter(|g| g.bus_id == bus.id && g.in_service)
        .collect();
    for g in &machines {
        let id = ids.next_id("Machine");
        let term = ids.next_id("Terminal");
        write_cim_machine(out, &id, &term, g, bus.bus_type == BusType::Slack);
        terminals.push(term);
    }
    if bus.bus_type == BusType::Slack && machines.is_empty() {
        // The importer derives the slack bus from an EquivalentInjection, so a
        // slack bus with no machine needs a synthetic one to round-trip.
        let id = ids.next_id("Machine");
        let term = ids.next_id("Terminal");
        let synthetic = Generator::new(
            0,
            format!("Slack at bus {}", bus.id),
            GeneratorType::Other,
            1.0,
            -1.0,
        )
        .at_bus(bus.id);
        write_cim_machine(out, &id, &term, &synthetic, true);
        terminals.push(term);
    }
    if bus.load_mw != 0.0 || bus.load_mvar != 0.0 {
        let term = ids.next_id("Terminal");
        let id = ids.next_id("EnergyConsumer");
        write_object_header(out, "EnergyConsumer", &id, &format!("Load at bus {}", bus.id));
        let _ = writeln!(out, "    <cim:EnergyConsumer.p>{}</cim:EnergyConsumer.p>", bus.load_mw);
        let _ = writeln!(out, "    <cim:EnergyConsumer.q>{}</cim:EnergyConsumer.q>", bus.load_mvar);
        let _ = writeln!(out, "    <cim:EnergyConsumer.Terminals rdf:resource=\"{term}\"/>");
        let _ = writeln!(out, "  </cim:EnergyConsumer>");
        write_terminal(out, &term, &id, bus.id, "EnergyConsumer");
        terminals.push(term);
    }

    let _ = writeln!(out, "  <cim:ConnectivityNode rdf:about=\"{}\">", node_id(bus.id));
    let _ = writeln!(
        out,
        "    <cim:IdentifiedObject.name>{}</cim:IdentifiedObject.name>",
        xml_escape(&bus.name)
    );
    let _ = writeln!(
        out,
        "    <cim:ConnectivityNode.Container rdf:resource=\"urn:tpt-energy:VoltageLevel:{}\"/>",
        bus.base_kv
    );
    for term in &terminals {
        let _ = writeln!(out, "    <cim:ConnectivityNode.Terminals rdf:resource=\"{term}\"/>");
    }
    let _ = writeln!(out, "  </cim:ConnectivityNode>");
}

/// Emit a machine as a `RotatingMachine`, or an `EquivalentInjection` at the
/// slack bus (which is what the importer reads back as a slack bus).
fn write_cim_machine(
    out: &mut String,
    id: &str,
    term: &str,
    g: &Generator,
    is_slack: bool,
) {
    if is_slack {
        write_object_header(out, "EquivalentInjection", id, g.name.as_str());
        let _ = writeln!(out, "    <cim:EquivalentInjection.p>{}</cim:EquivalentInjection.p>", g.p_schedule_mw);
        let _ = writeln!(out, "    <cim:EquivalentInjection.q>{}</cim:EquivalentInjection.q>", g.q_min_mvar);
        let _ = writeln!(out, "    <cim:EquivalentInjection.maxP>{}</cim:EquivalentInjection.maxP>", g.p_max_mw);
        let _ = writeln!(out, "    <cim:EquivalentInjection.minP>{}</cim:EquivalentInjection.minP>", g.p_min_mw);
        let _ = writeln!(out, "    <cim:EquivalentInjection.Terminals rdf:resource=\"{term}\"/>");
        let _ = writeln!(out, "  </cim:EquivalentInjection>");
        write_terminal(out, term, id, g.bus_id, "EquivalentInjection");
    } else {
        write_object_header(out, "RotatingMachine", id, g.name.as_str());
        let _ = writeln!(out, "    <cim:RotatingMachine.p>{}</cim:RotatingMachine.p>", g.p_schedule_mw);
        let _ = writeln!(out, "    <cim:RotatingMachine.q>{}</cim:RotatingMachine.q>", g.q_min_mvar);
        let _ = writeln!(out, "    <cim:RotatingMachine.maxOperatingP>{}</cim:RotatingMachine.maxOperatingP>", g.p_max_mw);
        let _ = writeln!(out, "    <cim:RotatingMachine.minOperatingP>{}</cim:RotatingMachine.minOperatingP>", g.p_min_mw);
        let _ = writeln!(
            out,
            "    <cim:RotatingMachine.regulatedControl.targetValue>{}</cim:RotatingMachine.regulatedControl.targetValue>",
            g.voltage_setpoint_pu
        );
        let _ = writeln!(out, "    <cim:RotatingMachine.Terminals rdf:resource=\"{term}\"/>");
        let _ = writeln!(out, "  </cim:RotatingMachine>");
        write_terminal(out, term, id, g.bus_id, "RotatingMachine");
    }
}
/// Emit one `ACLineSegment` with its two terminals.
fn write_cim_line(out: &mut String, ids: &mut Ids, system: &EnergySystem, br: &Branch) {
    let id = ids.next_id("ACLineSegment");
    let t1 = ids.next_id("Terminal");
    let t2 = ids.next_id("Terminal");
    write_object_header(out, "ACLineSegment", &id, &br.name);
    let _ = writeln!(out, "    <cim:ACLineSegment.r>{}</cim:ACLineSegment.r>", br.resistance_pu);
    let _ = writeln!(out, "    <cim:ACLineSegment.x>{}</cim:ACLineSegment.x>", br.reactance_pu);
    let _ = writeln!(out, "    <cim:ACLineSegment.b>{}</cim:ACLineSegment.b>", br.susceptance_pu);
    let _ = writeln!(
        out,
        "    <cim:ACLineSegment.inService>{}</cim:ACLineSegment.inService>",
        u8::from(br.in_service)
    );
    let base_kv = system
        .buses
        .iter()
        .find(|b| b.id == br.from_bus)
        .map_or(0.0, |b| b.base_kv);
    let amps = if base_kv > 0.0 {
        br.rating_mva * 1000.0 / base_kv
    } else {
        0.0
    };
    let _ = writeln!(out, "    <cim:ACLineSegment.CurrentLimit>{amps:.4}</cim:ACLineSegment.CurrentLimit>");
    let _ = writeln!(out, "    <cim:ACLineSegment.Terminals rdf:resource=\"{t1}\"/>");
    let _ = writeln!(out, "    <cim:ACLineSegment.Terminals rdf:resource=\"{t2}\"/>");
    let _ = writeln!(out, "  </cim:ACLineSegment>");
    write_terminal(out, &t1, &id, br.from_bus, "ACLineSegment");
    write_terminal(out, &t2, &id, br.to_bus, "ACLineSegment");
}
