//! Benchmarks for NREL SPA solar position calculation.

use criterion::{criterion_group, criterion_main, Criterion};

fn _placeholder() {}

fn bench_spa(c: &mut Criterion) {
    c.bench_function("solar_spa_placeholder", |b| b.iter(_placeholder));
}

criterion_group!(benches, bench_spa);
criterion_main!(benches);
