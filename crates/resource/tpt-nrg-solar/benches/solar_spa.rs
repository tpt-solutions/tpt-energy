//! Benchmark: solar-position calculation (SPA-equivalent path) and PV
//! output over a full year at hourly resolution.

use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;
use tpt_nrg_solar::{PvPlant, PvPlantConfig, SolarModel};

fn bench_spa(c: &mut Criterion) {
    let site = SolarModel::new(40.0, -105.0, 1600.0, -7.0);
    let start = chrono::Utc::now();

    let mut group = c.benchmark_group("solar");
    group.measurement_time(Duration::from_secs(5));
    group.bench_function("solar_position_8760h", |b| {
        b.iter(|| {
            for h in 0..8760 {
                let t = start + chrono::Duration::hours(h);
                let _pos = site.solar_position(t);
            }
        });
    });

    let config = PvPlantConfig::new(100.0, 30.0, 180.0, 25.0);
    let plant = PvPlant::new(site, config);
    group.bench_function("pv_output_8760h", |b| {
        b.iter(|| {
            for h in 0..8760 {
                let t = start + chrono::Duration::hours(h);
                let _out = plant.output_at(t);
            }
        });
    });
    group.finish();
}

criterion_group!(benches, bench_spa);
criterion_main!(benches);
