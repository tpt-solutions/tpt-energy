//! Single-line diagram and heatmap rendering.

use std::fmt::Write as _;

use tpt_nrg_core::{BusType, EnergySystem};
use tpt_nrg_powerflow::PowerFlowResult;

use crate::heatmap::{loading_band, voltage_band, Band, LoadingBand};
use crate::layout::{self, Layout, Point};

/// Rendering options.
///
/// The four `show_*` flags are independent overlays; grouping them behind a
/// single enum would mean a four-variant enum and a `match` at every call site,
/// so the struct is a plain set of toggles.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct VizOptions {
    /// Document title, also used as the SVG `<title>`. An empty string means
    /// "use the system name".
    pub title: String,
    /// Bus label text; `None` uses the bus id.
    pub show_bus_ids: bool,
    /// Draw the per-branch MW flow label at the branch midpoint.
    pub show_flow_labels: bool,
    /// Draw the voltage / loading legend.
    pub show_legend: bool,
    /// Draw a bus marker per attached generator.
    pub show_generators: bool,
    /// Background colour; `None` leaves the canvas transparent.
    pub background: Option<String>,
}

impl Default for VizOptions {
    fn default() -> Self {
        Self {
            title: String::new(),
            show_bus_ids: true,
            show_flow_labels: true,
            show_legend: true,
            show_generators: true,
            background: Some("#ffffff".to_string()),
        }
    }
}

/// Gap between the diagram and the legend, in user-space units.
const LEGEND_GAP: f64 = 40.0;
/// Vertical distance between the two legend rows.
const LEGEND_ROW_GAP: f64 = 44.0;
/// Height reserved for the legend block.
const LEGEND_HEIGHT: f64 = 88.0;
/// Horizontal distance between legend swatches.
const LEGEND_SWATCH_WIDTH: f64 = 150.0;
/// Radius of a bus marker.
const BUS_RADIUS: f64 = 9.0;
/// Font size for labels.
const FONT_SIZE: f64 = 11.0;

/// Render `system` (and an optional solved `result`) as one SVG document.
///
/// The document contains the single-line diagram, a voltage heatmap swatch on
/// every bus, and — unless disabled — a legend and MW flow labels.
#[must_use]
pub fn render(
    system: &EnergySystem,
    result: Option<&PowerFlowResult>,
    options: &VizOptions,
) -> String {
    let layout = layout::compute(system);
    let mut out = String::with_capacity(16 * 1024);

    let (diagram_w, diagram_h) = (layout.width, layout.height);
    let legend_w = if options.show_legend {
        LEGEND_GAP + layout::count_to_f64(Band::all().len()) * 150.0
    } else {
        0.0
    };
    let width = diagram_w + legend_w;
    let height = diagram_h
        + if options.show_legend {
            LEGEND_HEIGHT
        } else {
            0.0
        };

    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width:.0}\" height=\"{height:.0}\" \
         viewBox=\"0 0 {width:.0} {height:.0}\" font-family=\"sans-serif\">"
    );
    let title = if options.title.is_empty() {
        &system.name
    } else {
        &options.title
    };
    let _ = writeln!(out, "  <title>{}</title>", escape(title));
    if let Some(bg) = &options.background {
        let _ = writeln!(
            out,
            "  <rect width=\"{width:.0}\" height=\"{height:.0}\" fill=\"{bg}\"/>"
        );
    }

    draw_branches(&mut out, system, result, &layout, options);
    draw_buses(&mut out, system, result, &layout, options);
    if options.show_legend {
        draw_legend(&mut out, diagram_w, diagram_h);
    }
    out.push_str("</svg>\n");
    out
}

/// Voltage magnitude of a bus, from the solution when available and from the
/// scheduled setpoint otherwise.
fn bus_voltage(system: &EnergySystem, result: Option<&PowerFlowResult>, bus: usize) -> f64 {
    let index = system.buses.iter().position(|b| b.id == bus);
    match (result, index) {
        (Some(r), Some(i)) => r
            .bus_voltage_magnitude_pu
            .get(i)
            .copied()
            .unwrap_or(f64::NAN),
        _ => system
            .buses
            .iter()
            .find(|b| b.id == bus)
            .map_or(f64::NAN, |b| b.voltage_magnitude_pu),
    }
}

/// Draw the branch lines, coloured by loading.
fn draw_branches(
    out: &mut String,
    system: &EnergySystem,
    result: Option<&PowerFlowResult>,
    layout: &Layout,
    options: &VizOptions,
) {
    for br in &system.branches {
        let a = layout.position(br.from_bus);
        let b = layout.position(br.to_bus);
        let flow = result.and_then(|r| {
            r.branch_flows
                .iter()
                .find(|f| f.id == br.id)
                .map(|f| f.loading_fraction)
        });
        let band = match flow {
            Some(fraction) => loading_band(fraction),
            None if !br.in_service => LoadingBand::Unknown,
            None => LoadingBand::Normal,
        };
        let dash = if br.in_service {
            ""
        } else {
            " stroke-dasharray=\"6 4\""
        };
        let _ = writeln!(
            out,
            "  <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" \
             stroke=\"{}\" stroke-width=\"3\"{}/>",
            a.x,
            a.y,
            b.x,
            b.y,
            band.color(),
            dash
        );
        if options.show_flow_labels {
            if let Some(mw) = branch_mw(result, br.id) {
                let mid = Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
                let _ = writeln!(
                    out,
                    "  <text x=\"{:.1}\" y=\"{:.1}\" font-size=\"{FONT_SIZE}\" \
                     text-anchor=\"middle\" fill=\"#374151\">{mw:.1} MW</text>",
                    mid.x,
                    mid.y - 4.0
                );
            }
        }
    }
}

/// Active power flow at the "from" end of a branch, if a solution is present.
fn branch_mw(result: Option<&PowerFlowResult>, branch: usize) -> Option<f64> {
    result?
        .branch_flows
        .iter()
        .find(|f| f.id == branch)
        .map(|f| f.p_from_mw)
}

/// Draw the bus markers, voltage heatmap fill, and labels.
fn draw_buses(
    out: &mut String,
    system: &EnergySystem,
    result: Option<&PowerFlowResult>,
    layout: &Layout,
    options: &VizOptions,
) {
    for bus in &system.buses {
        let p = layout.position(bus.id);
        let voltage = bus_voltage(system, result, bus.id);
        let band = voltage_band(voltage);
        let _ = writeln!(
            out,
            "  <circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{BUS_RADIUS:.0}\" fill=\"{}\" \
             stroke=\"#111827\" stroke-width=\"1.5\"/>",
            p.x,
            p.y,
            band.color()
        );
        if bus.bus_type == BusType::Slack {
            let _ = writeln!(
                out,
                "  <circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.0}\" fill=\"none\" \
                 stroke=\"#111827\" stroke-width=\"1.5\"/>",
                p.x,
                p.y,
                BUS_RADIUS + 4.0
            );
        }
        if options.show_bus_ids {
            let _ = writeln!(
                out,
                "  <text x=\"{:.1}\" y=\"{:.1}\" font-size=\"{FONT_SIZE}\" \
                 text-anchor=\"middle\" fill=\"#111827\">{}</text>",
                p.x,
                p.y - BUS_RADIUS - 5.0,
                bus.id
            );
            let _ = writeln!(
                out,
                "  <text x=\"{:.1}\" y=\"{:.1}\" font-size=\"{FONT_SIZE}\" \
                 text-anchor=\"middle\" fill=\"#374151\">{:.3} pu</text>",
                p.x,
                p.y + BUS_RADIUS + 12.0,
                voltage
            );
        }
        if options.show_generators {
            draw_generator_marker(out, system, bus.id, p);
        }
    }
}

/// Draw a capacity marker below a bus that carries generation.
fn draw_generator_marker(out: &mut String, system: &EnergySystem, bus: usize, p: Point) {
    let mw: f64 = system
        .generators
        .iter()
        .filter(|g| g.bus_id == bus && g.in_service)
        .map(|g| g.p_max_mw)
        .sum();
    if mw <= 0.0 {
        return;
    }
    let _ = writeln!(
        out,
        "  <text x=\"{:.1}\" y=\"{:.1}\" font-size=\"{FONT_SIZE}\" text-anchor=\"middle\" \
         fill=\"#166534\">G {:.0} MW</text>",
        p.x,
        p.y + BUS_RADIUS + 24.0,
        mw
    );
}

/// Draw both legends beside the diagram: bus voltage fill, and branch loading.
fn draw_legend(out: &mut String, diagram_w: f64, top: f64) {
    let _ = writeln!(
        out,
        "  <g transform=\"translate({:.0},{:.0})\">",
        diagram_w + LEGEND_GAP,
        top
    );
    let _ = writeln!(
        out,
        "    <text x=\"0\" y=\"0\" font-size=\"{FONT_SIZE}\" fill=\"#111827\">Bus fill: voltage (pu)</text>"
    );
    draw_swatches(
        out,
        14.0,
        Band::all().iter().map(|b| (b.color(), b.label())),
    );
    let _ = writeln!(
        out,
        "    <text x=\"0\" y=\"{LEGEND_ROW_GAP:.0}\" font-size=\"{FONT_SIZE}\" fill=\"#111827\">\
         Branch colour: loading vs rating</text>"
    );
    draw_swatches(
        out,
        LEGEND_ROW_GAP + 14.0,
        LoadingBand::all().iter().map(|b| (b.color(), b.label())),
    );
    let _ = writeln!(out, "  </g>");
}

/// Draw one row of colour swatches with labels.
fn draw_swatches<'a, I>(out: &mut String, y: f64, bands: I)
where
    I: Iterator<Item = (&'a str, &'a str)>,
{
    for (i, (color, label)) in bands.enumerate() {
        let x = layout::count_to_f64(i) * LEGEND_SWATCH_WIDTH;
        let _ = writeln!(
            out,
            "    <rect x=\"{x:.0}\" y=\"{y:.0}\" width=\"14\" height=\"14\" fill=\"{color}\"/>"
        );
        let _ = writeln!(
            out,
            "    <text x=\"{:.0}\" y=\"{:.0}\" font-size=\"{FONT_SIZE}\" fill=\"#374151\">{label}</text>",
            x + 20.0,
            y + 11.0
        );
    }
}

/// Escape the five XML entities in element text.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
