//! `tpt-nrg viz` — option construction for the SVG renderer.

use tpt_nrg_viz::VizOptions;

use crate::VizArgs;

/// Build renderer options from the command line.
///
/// The document title is left empty so that the renderer falls back to the
/// system name, which is more informative than a fixed string.
#[must_use]
pub fn options(args: &VizArgs) -> VizOptions {
    VizOptions {
        title: String::new(),
        show_bus_ids: true,
        show_flow_labels: !args.no_flow_labels,
        show_legend: !args.no_legend,
        show_generators: true,
        background: Some("#ffffff".to_string()),
    }
}
