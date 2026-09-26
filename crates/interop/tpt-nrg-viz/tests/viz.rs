//! Rendering tests: the output must be well-formed, deterministic, and driven
//! by the power-flow result.

use tpt_nrg_core::{Branch, Bus, BusType, EnergySystem, Generator, GeneratorType};
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};
use tpt_nrg_viz::{layout, render, Band, VizOptions};

/// A three-bus system with one out-of-service branch.
fn three_bus() -> EnergySystem {
    let mut sys = EnergySystem::new("viz", "Viz demo", 100.0, 60.0);
    sys.add_bus(Bus::new(1, "Slack", BusType::Slack).with_voltage_pu(1.06, 0.0))
        .unwrap();
    sys.add_bus(Bus::new(2, "B2", BusType::Pv).with_voltage_pu(1.02, 0.0))
        .unwrap();
    sys.add_bus(
        Bus::new(3, "B3", BusType::Pq)
            .with_voltage_pu(0.97, 0.0)
            .with_load(40.0, 15.0),
    )
    .unwrap();
    sys.add_branch(Branch::new(1, "L12", 1, 2, 0.01, 0.1).with_rating(200.0))
        .unwrap();
    sys.add_branch(Branch::new(2, "L23", 2, 3, 0.02, 0.1).with_in_service(false))
        .unwrap();
    sys.add_generator(Generator::new(1, "G1", GeneratorType::Thermal, 80.0, 0.0).at_bus(1))
        .unwrap();
    sys
}

#[test]
fn render_produces_well_formed_svg() {
    let sys = three_bus();
    let svg = render(&sys, None, &VizOptions::default());
    assert!(svg.starts_with("<svg"));
    assert!(svg.trim_end().ends_with("</svg>"));
    assert!(svg.contains("<title>Viz demo</title>"));
    // One circle per bus plus the slack ring.
    assert_eq!(svg.matches("<circle").count(), 4);
    // One line per branch.
    assert_eq!(svg.matches("<line").count(), 2);
}

#[test]
fn render_is_deterministic() {
    let sys = three_bus();
    let options = VizOptions::default();
    let a = render(&sys, None, &options);
    let b = render(&sys, None, &options);
    assert_eq!(a, b, "the same input must render byte-identically");
}

#[test]
fn out_of_service_branches_are_dashed() {
    let sys = three_bus();
    let svg = render(&sys, None, &VizOptions::default());
    assert_eq!(
        svg.matches("stroke-dasharray").count(),
        1,
        "exactly the out-of-service branch is dashed"
    );
}

#[test]
fn bus_fill_reflects_the_voltage_band() {
    let sys = three_bus();
    let svg = render(&sys, None, &VizOptions::default());
    // Bus 3 is scheduled at 0.97 pu, which is the `Low` band.
    assert!(
        svg.contains(Band::Low.color()),
        "a 0.97 pu bus is drawn in the low-voltage colour"
    );
    assert!(svg.contains("0.970 pu"), "the numeric voltage is labelled");
}

#[test]
fn legend_can_be_disabled() {
    let sys = three_bus();
    let with = render(&sys, None, &VizOptions::default());
    let without = render(
        &sys,
        None,
        &VizOptions {
            show_legend: false,
            ..VizOptions::default()
        },
    );
    assert!(with.contains("1.05 - 1.10 pu"));
    assert!(
        with.contains("95 - 100%"),
        "the loading legend is drawn too"
    );
    assert!(!without.contains("1.05 - 1.10 pu"));
    assert!(!without.contains("95 - 100%"));
    assert!(without.len() < with.len());
}

#[test]
fn titles_are_escaped() {
    let mut sys = three_bus();
    sys.name = "A & B <case>".to_string();
    let svg = render(
        &sys,
        None,
        &VizOptions {
            title: "A & B <case>".to_string(),
            ..VizOptions::default()
        },
    );
    assert!(svg.contains("A &amp; B &lt;case&gt;"));
    assert!(!svg.contains("<case>"));
}

#[test]
fn solved_results_drive_labels_and_loading() {
    let sys = EnergySystem::from_json(include_str!("../../../../test-data/ieee/ieee14.json"))
        .expect("load ieee14");
    let result = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson)
        .solve(&sys)
        .expect("solve ieee14");
    let svg = render(&sys, Some(&result), &VizOptions::default());
    assert!(
        svg.contains("MW"),
        "flow labels are drawn from the solution"
    );
    // The golden solution has all bus voltages inside the normal band.
    assert!(result
        .bus_voltage_magnitude_pu
        .iter()
        .all(|v| (0.95..=1.10).contains(v)));
}

#[test]
fn empty_system_renders() {
    let sys = EnergySystem::new("empty", "Empty", 100.0, 60.0);
    let svg = render(&sys, None, &VizOptions::default());
    assert!(svg.starts_with("<svg"));
    assert!(svg.trim_end().ends_with("</svg>"));
    assert!(!svg.contains("<circle"));
}

#[test]
fn layout_places_every_bus() {
    let sys = three_bus();
    let l = layout::compute(&sys);
    assert_eq!(l.positions.len(), 3);
    for bus in &sys.buses {
        assert!(l.positions.contains_key(&bus.id), "bus {} placed", bus.id);
    }
    assert!(l.width > 0.0 && l.height > 0.0);
    // The slack bus is leftmost in its column, so the diagram reads
    // left to right; buses in the slack's own column share its x.
    let slack = l.position(1);
    for bus in &sys.buses {
        assert!(
            l.position(bus.id).x >= slack.x - f64::EPSILON,
            "bus {} at {} is left of slack at {}",
            bus.id,
            l.position(bus.id).x,
            slack.x
        );
    }
}

#[test]
fn layout_handles_disconnected_buses() {
    let mut sys = three_bus();
    sys.add_bus(Bus::new(9, "Island", BusType::Isolated))
        .unwrap();
    let l = layout::compute(&sys);
    assert_eq!(l.positions.len(), 4, "an isolated bus is still placed");
    assert!(l.positions.contains_key(&9));
}

#[test]
fn layout_handles_an_empty_system() {
    let sys = EnergySystem::new("empty", "Empty", 100.0, 60.0);
    let l = layout::compute(&sys);
    assert!(l.positions.is_empty());
    assert!(l.width > 0.0);
}
