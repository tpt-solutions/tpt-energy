//! Benchmark: Newton–Raphson AC power flow on synthetic meshed systems.
//!
//! Builds wrap-around lattice systems of increasing size (every bus
//! loaded, every third bus generating) and measures full NR solves with
//! Q-limit enforcement enabled.
//!
//! Builds ring-lattice systems of increasing size (every bus loaded, every
//! third bus generating) and measures full NR solves with Q-limit
//! enforcement enabled.

use criterion::{criterion_group, criterion_main, Criterion};
use tpt_nrg_core::{Branch, Bus, BusType, EnergySystem, Generator, GeneratorType};
use tpt_nrg_powerflow::{PowerFlowMethod, PowerFlowSolver};

/// Meshed lattice: `n` buses in a ring, each also tied to the bus six
/// positions ahead (wrap-around), keeping the electrical distance short.
fn lattice_system(n: usize) -> EnergySystem {
    let mut sys = EnergySystem::new("bench", "Bench ring", 100.0, 60.0);
    for i in 1..=n {
        let bus_type = if i == 1 {
            BusType::Slack
        } else if i % 3 == 0 {
            BusType::Pv
        } else {
            BusType::Pq
        };
        let mut bus = Bus::new(i, format!("B{i}"), bus_type)
            .with_load(8.0, 3.0)
            .with_voltage_pu(1.0, 0.0);
        if bus_type != BusType::Pq {
            bus.voltage_magnitude_pu = 1.02;
        }
        sys.add_bus(bus).unwrap();
    }
    let mut branch_id = 1;
    for i in 1..=n {
        let j = (i % n) + 1;
        sys.add_branch(Branch::new(
            branch_id,
            format!("L{branch_id}"),
            i,
            j,
            0.01,
            0.06,
        ))
        .unwrap();
        branch_id += 1;
        let k = ((i + 6) % n) + 1;
        if k != i {
            sys.add_branch(Branch::new(
                branch_id,
                format!("T{branch_id}"),
                i,
                k,
                0.01,
                0.06,
            ))
            .unwrap();
            branch_id += 1;
        }
    }
    for i in (3..=n).step_by(3) {
        let mut gen =
            Generator::new(i, format!("G{i}"), GeneratorType::Thermal, 40.0, 0.0).at_bus(i);
        gen.q_max_mvar = 30.0;
        gen.q_min_mvar = -30.0;
        sys.add_generator(gen).unwrap();
    }
    sys
}

fn bench_newton_raphson(c: &mut Criterion) {
    let mut group = c.benchmark_group("newton_raphson");
    for n in [30, 100, 200] {
        let system = lattice_system(n);
        let solver = PowerFlowSolver::new(PowerFlowMethod::NewtonRaphson)
            .with_tolerance(1e-10)
            .with_max_iterations(100);
        group.bench_function(format!("nr_{n}_bus"), |b| {
            b.iter(|| solver.solve(&system).expect("bench solve must converge"));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_newton_raphson);
criterion_main!(benches);
