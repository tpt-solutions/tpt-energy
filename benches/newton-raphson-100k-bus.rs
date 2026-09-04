//! Benchmarks for Newton–Raphson power flow on large test systems.

use criterion::{criterion_group, criterion_main, Criterion};

fn _placeholder() {}

fn bench_newton_raphson(c: &mut Criterion) {
    c.bench_function("newton_raphson_placeholder", |b| b.iter(_placeholder));
}

criterion_group!(benches, bench_newton_raphson);
criterion_main!(benches);
