//! Benchmarks for MILP unit commitment.

use criterion::{criterion_group, criterion_main, Criterion};

fn _placeholder() {}

fn bench_uc(c: &mut Criterion) {
    c.bench_function("unit_commitment_placeholder", |b| b.iter(_placeholder));
}

criterion_group!(benches, bench_uc);
criterion_main!(benches);
