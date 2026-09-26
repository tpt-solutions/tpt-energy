//! Benchmark: priority-list unit commitment over a 24-hour horizon with
//! fleets of increasing size.

use criterion::{criterion_group, criterion_main, Criterion};
use tpt_nrg_core::{Bus, BusType, CostCurve, EnergySystem, Generator, GeneratorType};
use tpt_nrg_unit_commitment::unit_commitment;

/// Convert a bounded remainder to `f64` (values 0..=6, exact in `f64`).
#[allow(clippy::cast_precision_loss)]
fn rem_f64(value: usize) -> f64 {
    value as f64
}

fn fleet_system(n_units: usize) -> EnergySystem {
    let mut sys = EnergySystem::new("uc-bench", "UC bench", 100.0, 60.0);
    for i in 1..=n_units {
        sys.add_bus(Bus::new(i, format!("B{i}"), BusType::Pv))
            .unwrap();
        let p_max = 50.0 + rem_f64(i % 5) * 25.0;
        let mc = 10.0 + rem_f64(i % 7) * 5.0;
        sys.add_generator(
            Generator::new(
                i,
                format!("G{i}"),
                GeneratorType::Thermal,
                p_max,
                p_max * 0.4,
            )
            .at_bus(i)
            .with_cost_curve(CostCurve::piecewise(
                mc,
                p_max,
                mc * p_max * 0.4,
                mc * p_max,
            )),
        )
        .unwrap();
    }
    sys
}

fn bench_unit_commitment(c: &mut Criterion) {
    let load: Vec<f64> = (0..24)
        .map(|h| 300.0 + 150.0 * (rem_f64(h) * std::f64::consts::PI / 12.0).sin())
        .collect();
    let mut group = c.benchmark_group("unit_commitment");
    for n in [10, 30, 100] {
        let system = fleet_system(n);
        group.bench_function(format!("uc_24h_{n}_units"), |b| {
            b.iter(|| unit_commitment(&system, &load));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_unit_commitment);
criterion_main!(benches);
