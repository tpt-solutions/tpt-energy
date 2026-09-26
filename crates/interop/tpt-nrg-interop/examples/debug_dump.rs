//! Temporary debug harness: round-trip a small system through every format.

use tpt_nrg_core::{Branch, Bus, BusType, EnergySystem, Generator, GeneratorType};
use tpt_nrg_interop::{cim, matpower, psse, tabular};

fn build() -> EnergySystem {
    let mut sys = EnergySystem::new("demo", "Demo", 100.0, 60.0);
    sys.add_bus(Bus::new(1, "B1", BusType::Slack).with_voltage_pu(1.06, 0.0))
        .unwrap();
    sys.add_bus(Bus::new(2, "B2", BusType::Pv).with_voltage_pu(1.02, 0.0))
        .unwrap();
    sys.add_bus(Bus::new(3, "B3", BusType::Pq).with_load(40.0, 15.0))
        .unwrap();
    sys.add_branch(Branch::new(1, "L12", 1, 2, 0.01, 0.1))
        .unwrap();
    sys.add_branch(Branch::new(2, "L23", 2, 3, 0.02, 0.1).with_in_service(false))
        .unwrap();
    sys.add_generator(Generator::new(1, "G1", GeneratorType::Wind, 80.0, 0.0).at_bus(1))
        .unwrap();
    sys.add_generator(Generator::new(2, "G2", GeneratorType::Solar, 50.0, 0.0).at_bus(2))
        .unwrap();
    sys
}

fn report(name: &str, text: &str, back: Result<EnergySystem, String>) {
    match back {
        Ok(sys) => println!(
            "{name}: buses={} branches={} gens={} ids={:?}",
            sys.buses.len(),
            sys.branches.len(),
            sys.generators.len(),
            sys.buses.iter().map(|b| b.id).collect::<Vec<_>>()
        ),
        Err(e) => println!("{name}: ERROR {e}\n{text}"),
    }
}

fn main() {
    let sys = build();

    let text = matpower::to_matpower(&sys).expect("write matpower");
    let back = matpower::from_matpower(&text).map_err(|e| e.to_string());
    report("matpower", &text, back);

    let text = psse::to_psse(&sys).expect("write psse");
    println!("--- psse ---\n{text}");
    let back = psse::from_psse(&text).map_err(|e| e.to_string());
    report("psse", &text, back);

    let text = tabular::to_flat_csv(&sys).expect("write csv");
    println!("--- csv ---\n{text}");
    let back = tabular::from_flat_csv(&text).map_err(|e| e.to_string());
    report("csv", &text, back);

    let text = cim::to_cim(&sys).expect("write cim");
    println!("--- cim ---\n{text}");
    let back = cim::from_cim(&text).map_err(|e| e.to_string());
    report("cim", &text, back);
}
