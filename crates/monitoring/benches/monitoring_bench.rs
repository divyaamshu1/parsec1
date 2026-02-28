#![allow(dead_code, unused_imports)]

//! Monitoring benchmarks

use criterion::{criterion_group, criterion_main, Criterion, black_box};
use std::time::Duration;

fn bench_profiler(c: &mut Criterion) {
    let mut group = c.benchmark_group("profiler");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    group.bench_function("noop", |b| {
        b.iter(|| { black_box(42) })
    });

    group.finish();
}

fn bench_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    group.bench_function("noop", |b| {
        b.iter(|| { black_box(42) })
    });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(10))
        .sample_size(50);
    targets = bench_profiler, bench_metrics
);
criterion_main!(benches);