//! Round-trip and regression tests for every exchange format.

use std::path::Path;

use tpt_nrg_core::{Branch, Bus, BusType, EnergySystem, Generator, GeneratorType};
use tpt_nrg_interop::{cim, matpower, psse, tabular, Format, InteropErrorKind};

/// A minimal but complete two-bus system used across the tests.
fn two_bus() -> EnergySystem {
    let mut sys = EnergySystem::new("demo", "Two-bus demo", 100.0, 60.0);
    sys.add_bus(
        Bus::new(1, "Slack", BusType::Slack)
            .with_voltage_pu(1.06, 0.0)
            .with_base_kv(132.0),
    )
    .unwrap();
    sys.add_bus(
        Bus::new(2, "Load", BusType::Pq)
            .with_voltage_pu(1.0, 0.0)
            .with_base_kv(132.0)
            .with_load(50.0, 20.0),
    )
    .unwrap();
    sys.add_branch(Branch::new(1, "L12", 1, 2, 0.01, 0.05).with_susceptance(0.02))
        .unwrap();
    sys.add_generator(
        Generator::new(1, "G1", GeneratorType::Thermal, 100.0, 10.0)
            .at_bus(1)
            .with_voltage_setpoint(1.06),
    )
    .unwrap();
    sys
}

/// A larger case with a PV bus and an out-of-service branch, to exercise the
/// paths the two-bus case does not.
fn three_bus() -> EnergySystem {
    let mut sys = EnergySystem::new("ieee3", "Three-bus demo", 100.0, 60.0);
    sys.add_bus(Bus::new(1, "B1", BusType::Slack).with_voltage_pu(1.06, 0.0))
        .unwrap();
    sys.add_bus(Bus::new(2, "B2", BusType::Pv).with_voltage_pu(1.02, 0.0))
        .unwrap();
    sys.add_bus(Bus::new(3, "B3", BusType::Pq).with_load(40.0, 15.0))
        .unwrap();
    sys.add_branch(Branch::new(1, "L12", 1, 2, 0.01, 0.1).with_rating(200.0))
        .unwrap();
    sys.add_branch(Branch::new(2, "L23", 2, 3, 0.02, 0.1).with_in_service(false))
        .unwrap();
    sys.add_generator(
        Generator::new(1, "G1", GeneratorType::Wind, 80.0, 0.0)
            .at_bus(1)
            .with_reactive_limits(-50.0, 50.0)
            .with_voltage_setpoint(1.06),
    )
    .unwrap();
    sys.add_generator(
        Generator::new(2, "G2", GeneratorType::Solar, 50.0, 0.0)
            .at_bus(2)
            .with_reactive_limits(-30.0, 30.0)
            .with_voltage_setpoint(1.02),
    )
    .unwrap();
    sys
}

#[test]
fn format_names_round_trip() {
    for (name, expected) in [
        ("json", Format::Json),
        ("YAML", Format::Yaml),
        ("yml", Format::Yaml),
        ("csv", Format::Csv),
        ("matpower", Format::Matpower),
        ("mpc", Format::Matpower),
        ("psse", Format::Psse),
        ("raw", Format::Psse),
        ("cim", Format::Cim),
        ("rdf", Format::Cim),
    ] {
        let parsed = Format::parse(name).expect("known format");
        assert_eq!(parsed, expected, "{name}");
        assert_eq!(parsed.to_string(), expected.as_str());
    }
    assert!(Format::parse("excel").is_err());
}

#[test]
fn matpower_round_trip_preserves_the_network() {
    let sys = three_bus();
    let text = matpower::to_matpower(&sys).expect("write matpower");
    let back = matpower::from_matpower(&text).expect("read matpower");
    assert_eq!(back.buses.len(), sys.buses.len());
    assert_eq!(back.branches.len(), sys.branches.len());
    assert_eq!(back.generators.len(), sys.generators.len());
    assert!((back.base_mva - sys.base_mva).abs() < 1e-9);
    for (a, b) in sys.buses.iter().zip(back.buses.iter()) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.bus_type, b.bus_type);
        assert!((a.voltage_magnitude_pu - b.voltage_magnitude_pu).abs() < 1e-6);
        assert!((a.voltage_angle_rad - b.voltage_angle_rad).abs() < 1e-6);
    }
    for (a, b) in sys.branches.iter().zip(back.branches.iter()) {
        assert_eq!(a.from_bus, b.from_bus);
        assert_eq!(a.to_bus, b.to_bus);
        assert!((a.resistance_pu - b.resistance_pu).abs() < 1e-9);
        assert!((a.reactance_pu - b.reactance_pu).abs() < 1e-9);
        assert!((a.susceptance_pu - b.susceptance_pu).abs() < 1e-9);
        assert_eq!(a.in_service, b.in_service);
    }
    assert!(!back.branches[1].in_service, "out-of-service flag survives");
}

#[test]
fn matpower_export_is_idempotent() {
    let sys = three_bus();
    let once = matpower::to_matpower(&sys).expect("write");
    let reparsed = matpower::from_matpower(&once).expect("read");
    let twice = matpower::to_matpower(&reparsed).expect("rewrite");
    assert_eq!(once, twice, "a second round trip must be byte-identical");
}

#[test]
fn matpower_rejects_a_non_case_file() {
    let err = matpower::from_matpower("this is not a case file").unwrap_err();
    assert_eq!(err.kind(), InteropErrorKind::Parse);
}

#[test]
fn matpower_requires_base_mva() {
    let text = "mpc.bus = [\n 1 3 0 0 0 0 1 1 0 100 1 1 1;\n];";
    let err = matpower::from_matpower(text).unwrap_err();
    assert_eq!(err.kind(), InteropErrorKind::MissingRecord);
}

#[test]
fn matpower_ignores_comments_and_blank_lines() {
    let text = "\
%% MATPOWER Case Format : Version 2
mpc.baseMVA = 100;

%% bus data
%   bus_i type Pd Qd Gs Bs area Vm Va baseKV zone Vmax Vmin
mpc.bus = [
  1  3  0  0  0  0  1  1.06  0  132  1  1.1  0.9;  % trailing comment
  2  1  0  0  0  0  1  1.0  0  132  1  1.1  0.9;
];

mpc.gen = [
 1  0  0  0  0  1.06  100  1  100  0;
];
mpc.branch = [
 1  2  0.01  0.05  0  0  0  0  0  0  1;
];
";
    let sys = matpower::from_matpower(text).expect("comments are stripped");
    assert_eq!(sys.buses.len(), 2);
    assert!((sys.buses[0].voltage_magnitude_pu - 1.06).abs() < 1e-9);
    assert!((sys.buses[0].base_kv - 132.0).abs() < 1e-9);
}

#[test]
fn psse_round_trip_preserves_the_network() {
    let sys = three_bus();
    let text = psse::to_psse(&sys).expect("write psse");
    assert!(text.contains("BEGIN BUS DATA"));
    let back = psse::from_psse(&text).expect("read psse");
    assert_eq!(back.buses.len(), 3);
    assert_eq!(back.branches.len(), 2);
    assert_eq!(back.generators.len(), 2);
    assert!((back.base_mva - 100.0).abs() < 1e-9);
    assert!((back.frequency_hz - 60.0).abs() < 1e-9);
    assert_eq!(
        back.buses
            .iter()
            .filter(|b| b.bus_type == BusType::Slack)
            .count(),
        1,
        "exactly one slack bus survives"
    );
    for (a, b) in sys.buses.iter().zip(back.buses.iter()) {
        assert!((a.voltage_magnitude_pu - b.voltage_magnitude_pu).abs() < 1e-4);
        assert!((a.load_mw - b.load_mw).abs() < 1e-4);
    }
    for (a, b) in sys.branches.iter().zip(back.branches.iter()) {
        assert_eq!(a.from_bus, b.from_bus);
        assert!((a.reactance_pu - b.reactance_pu).abs() < 1e-4);
    }
}

#[test]
fn psse_requires_bus_data() {
    let err = psse::from_psse("BEGIN SYSTEM DATA\nEND SYSTEM DATA\nEND\n").unwrap_err();
    assert_eq!(err.kind(), InteropErrorKind::MissingRecord);
}

#[test]
fn psse_tolerates_fortran_exponents() {
    // Derive the fixture from the exporter so the record layout is realistic,
    // then rewrite the numbers in Fortran `D` exponent form.
    let text = psse::to_psse(&three_bus()).expect("write psse");
    let text = text
        .replace("110.00", "1.10D+02")
        .replace("0.01000", "1.00D-02")
        .replace("0.10000", "1.00D-01");
    let sys = psse::from_psse(&text).expect("D exponents parse");
    assert_eq!(sys.buses.len(), 3);
    assert!((sys.buses[0].base_kv - 110.0).abs() < 1e-9);
    assert!((sys.branches[0].resistance_pu - 0.01).abs() < 1e-9);
    assert!((sys.branches[0].reactance_pu - 0.1).abs() < 1e-9);
    assert!((sys.buses[2].load_mw - 40.0).abs() < 1e-4);
}

#[test]
fn psse_reads_multiple_records_from_one_line() {
    // PSS/E wraps records at column 80, but a file may also put several
    // records on one physical line; the parser is line-agnostic.
    let text = psse::to_psse(&three_bus()).expect("write psse");
    let mut joined = String::new();
    let mut in_bus = false;
    for line in text.lines() {
        if line.starts_with("BEGIN BUS DATA") {
            in_bus = true;
            joined.push_str(line);
            joined.push('\n');
        } else if line.starts_with("END BUS DATA") {
            in_bus = false;
            joined.push('\n');
        } else if in_bus {
            joined.push_str(line);
            joined.push(' ');
        } else {
            joined.push_str(line);
            joined.push('\n');
        }
    }
    let sys = psse::from_psse(&joined).expect("records joined onto one line");
    assert_eq!(sys.buses.len(), 3);
    assert!((sys.frequency_hz - 60.0).abs() < 1e-9);
    assert!((sys.buses[2].load_mw - 40.0).abs() < 1e-4);
}

#[test]
fn psse_handles_the_v29_three_sub_block_record() {
    // A v29 bus record has three sub-blocks (bus, load, shunt) rather than the
    // six of v33; the dialect detector must pick 3.
    let text = "\
BEGIN SYSTEM DATA
TESTCASE 29 1.10 29 60 / 100.0 29 100.0 /
END SYSTEM DATA
BEGIN BUS DATA
  1  A  100.0  3  1  1  1  1.00  0.0  1.1  0.9 / 1 1 1 1 0.0 0.0 / /
  2  B  100.0  1  1  1  1  1.00  0.0  1.1  0.9 / 1 1 1 1 5.0 0.0 / /
END BUS DATA
END
";
    let sys = psse::from_psse(text).expect("v29 record shape");
    assert_eq!(sys.buses.len(), 2);
    assert!((sys.buses[1].load_mw - 5.0).abs() < 1e-9);
}

#[test]
fn cim_round_trip_preserves_the_network() {
    let sys = three_bus();
    let text = cim::to_cim(&sys).expect("write cim");
    assert!(text.contains("<rdf:RDF"));
    let back = cim::from_cim(&text).expect("read cim");
    let shape = format!(
        "buses={} branches={} gens={} loads={}",
        back.buses.len(),
        back.branches.len(),
        back.generators.len(),
        back.loads.len()
    );
    assert_eq!(back.buses.len(), 3, "{shape}");
    assert_eq!(back.branches.len(), 2, "{shape}");
    assert_eq!(back.generators.len(), 2, "{shape}");
    assert_eq!(
        back.buses
            .iter()
            .filter(|b| b.bus_type == BusType::Slack)
            .count(),
        1
    );
    for (a, b) in sys.buses.iter().zip(back.buses.iter()) {
        assert_eq!(a.id, b.id);
        assert!((a.load_mw - b.load_mw).abs() < 1e-6);
    }
    for (a, b) in sys.branches.iter().zip(back.branches.iter()) {
        assert_eq!(a.from_bus, b.from_bus);
        assert_eq!(a.to_bus, b.to_bus);
        assert!((a.resistance_pu - b.resistance_pu).abs() < 1e-6);
    }
}

#[test]
fn cim_requires_a_connectivity_node() {
    let err = cim::from_cim("<rdf:RDF xmlns:rdf=\"urn:x\"></rdf:RDF>").unwrap_err();
    assert_eq!(err.kind(), InteropErrorKind::MissingRecord);
}

#[test]
fn cim_rejects_malformed_xml() {
    let err = cim::from_cim("<rdf:RDF><ConnectivityNode>").unwrap_err();
    assert_eq!(err.kind(), InteropErrorKind::Parse);
}

#[test]
fn cim_strips_namespace_prefixes() {
    let prefixed = "<cim:RDF xmlns:cim=\"urn:x\"><cim:ConnectivityNode rdf:about=\"1\"/></cim:RDF>";
    let bare = "<RDF><ConnectivityNode rdf:about=\"1\"/></RDF>";
    for text in [prefixed, bare] {
        let parsed = cim::parse_xml(text).expect("well-formed");
        assert_eq!(parsed.children[0].name, "RDF");
        assert_eq!(parsed.children[0].children[0].name, "ConnectivityNode");
    }
}

#[test]
fn cim_synthesizes_a_slack_machine_when_needed() {
    let mut sys = two_bus();
    // Remove the only generator: the exporter must still produce a slack bus.
    sys.generators.clear();
    let text = cim::to_cim(&sys).expect("write cim");
    let back = cim::from_cim(&text).expect("read cim");
    assert_eq!(
        back.buses
            .iter()
            .filter(|b| b.bus_type == BusType::Slack)
            .count(),
        1
    );
}

#[test]
fn yaml_round_trip() {
    let sys = two_bus();
    let text = tabular::to_yaml(&sys).expect("write yaml");
    assert!(text.contains("buses:"));
    let back = tabular::from_yaml(&text).expect("read yaml");
    assert_eq!(back.buses.len(), 2);
    assert!((back.total_load_mw() - 50.0).abs() < 1e-9);
    assert_eq!(back.buses[0].bus_type, BusType::Slack);
}

#[test]
fn yaml_hand_written_case_parses() {
    let text = "\
id: hand
name: Hand written
base_mva: 100.0
frequency_hz: 50.0
buses:
  - id: 1
    name: Slack
    type: Slack
    voltage_magnitude_pu: 1.05
  - id: 2
    name: Load
    type: Pq
    load_mw: 25.0
    load_mvar: 5.0
branches:
  - id: 1
    name: L12
    from_bus: 1
    to_bus: 2
    resistance_pu: 0.02
    reactance_pu: 0.06
generators:
  - id: 1
    name: G1
    bus_id: 1
    type: Wind
    p_max_mw: 60.0
    p_min_mw: 0.0
";
    let sys = tabular::from_yaml(text).expect("hand-written yaml parses");
    assert_eq!(sys.id, "hand");
    assert!((sys.frequency_hz - 50.0).abs() < 1e-9);
    assert_eq!(sys.generators[0].generator_type, GeneratorType::Wind);
    assert!((sys.total_load_mw() - 25.0).abs() < 1e-9);
}

#[test]
fn flat_csv_round_trip() {
    let sys = three_bus();
    let text = tabular::to_flat_csv(&sys).expect("write csv");
    let back = tabular::from_flat_csv(&text).expect("read csv");
    assert_eq!(back.buses.len(), 3);
    assert_eq!(back.branches.len(), 2);
    assert_eq!(back.generators.len(), 2);
    for (a, b) in sys.buses.iter().zip(back.buses.iter()) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.bus_type, b.bus_type);
        assert!((a.load_mw - b.load_mw).abs() < 1e-6);
    }
    for (a, b) in sys.generators.iter().zip(back.generators.iter()) {
        assert_eq!(a.bus_id, b.bus_id);
        assert!((a.p_max_mw - b.p_max_mw).abs() < 1e-6);
    }
}

#[test]
fn flat_csv_requires_a_record_column() {
    let err = tabular::from_flat_csv("id,name\n1,a\n").unwrap_err();
    assert_eq!(err.kind(), InteropErrorKind::MissingRecord);
}

#[test]
fn flat_csv_rejects_an_unknown_record_type() {
    let text = "record,id,type\ntransformer,1,foo\n";
    let err = tabular::from_flat_csv(text).unwrap_err();
    assert_eq!(err.kind(), InteropErrorKind::BadValue);
}

#[test]
fn flat_csv_reads_a_system_row() {
    let text = "\
record,id,name,base_mva,frequency_hz,type,base_kv
system,flat,Flat demo,50.0,50.0,,
bus,1,Slack,,,Slack,33.0
bus,2,Load,,,Pq,33.0
";
    let sys = tabular::from_flat_csv(text).expect("read flat csv");
    assert_eq!(sys.id, "flat");
    assert_eq!(sys.name, "Flat demo");
    assert!((sys.base_mva - 50.0).abs() < 1e-9);
    assert!((sys.frequency_hz - 50.0).abs() < 1e-9);
    assert_eq!(sys.buses.len(), 2);
}

#[test]
fn from_text_dispatches_per_format() {
    let sys = two_bus();
    for format in [
        Format::Json,
        Format::Yaml,
        Format::Csv,
        Format::Matpower,
        Format::Psse,
        Format::Cim,
    ] {
        let text = tpt_nrg_interop::to_text(&sys, format).expect("write");
        let back = tpt_nrg_interop::from_text(&text, format)
            .unwrap_or_else(|e| panic!("{format} should re-import: {e}"));
        assert_eq!(back.buses.len(), 2, "{format}");
    }
}

#[test]
fn bundled_csv_directory_reads() {
    let dir = std::env::temp_dir().join("tpt-nrg-interop-csv-test");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    std::fs::write(
        dir.join("system.csv"),
        "id,name,base_mva,frequency_hz\nbundle,Bundled,50.0,50.0\n",
    )
    .expect("write system.csv");
    std::fs::write(
        dir.join("buses.csv"),
        "id,name,type,base_kv,load_mw,load_mvar\n\
1,Slack,Slack,33.0,0,0\n\
2,Load,Pq,33.0,10.5,2.5\n",
    )
    .expect("write buses.csv");
    std::fs::write(
        dir.join("branches.csv"),
        "id,name,from_bus,to_bus,resistance_pu,reactance_pu,rating_mva\n\
1,L12,1,2,0.01,0.05,60\n",
    )
    .expect("write branches.csv");
    std::fs::write(
        dir.join("generators.csv"),
        "id,name,bus_id,type,p_max_mw,p_min_mw,p_schedule_mw\n\
1,G1,1,Hydro,30.0,5.0,12.0\n",
    )
    .expect("write generators.csv");

    let sys = tabular::from_csv_dir(&dir).expect("read bundled csv");
    assert_eq!(sys.id, "bundle");
    assert!((sys.base_mva - 50.0).abs() < 1e-9);
    assert!((sys.frequency_hz - 50.0).abs() < 1e-9);
    assert_eq!(sys.buses.len(), 2);
    assert_eq!(sys.branches.len(), 1);
    assert_eq!(sys.generators[0].generator_type, GeneratorType::Hydro);
    assert!((sys.buses[1].load_mw - 10.5).abs() < 1e-9);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn bundled_csv_requires_buses() {
    let dir = std::env::temp_dir().join("tpt-nrg-interop-csv-empty");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let err = tabular::from_csv_dir(&dir).unwrap_err();
    assert_eq!(err.kind(), InteropErrorKind::MissingRecord);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn interop_reads_the_committed_ieee_cases() {
    let ieee = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("test-data")
        .join("ieee");
    for case in ["ieee14.json", "ieee30.json", "ieee57.json"] {
        let path = ieee.join(case);
        if !path.is_file() {
            continue;
        }
        let sys = EnergySystem::from_json_file(&path).expect("load committed case");
        let in_service = sys.branches.iter().filter(|b| b.in_service).count();
        for format in [Format::Matpower, Format::Psse, Format::Cim, Format::Yaml] {
            let text = tpt_nrg_interop::to_text(&sys, format)
                .unwrap_or_else(|e| panic!("{case} -> {format} failed: {e}"));
            let back = tpt_nrg_interop::from_text(&text, format)
                .unwrap_or_else(|e| panic!("{case} <- {format} failed: {e}"));
            assert_eq!(back.buses.len(), sys.buses.len(), "{case} {format}");
            assert_eq!(back.branches.len(), in_service, "{case} {format}");
        }
    }
}
