#![allow(dead_code, unused_imports)]

//! Terminal benchmarks

use criterion::{criterion_group, criterion_main, Criterion, black_box};
use std::time::Duration;

fn bench_terminal_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("terminal_creation");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    group.bench_function("create_terminal", |b| {
        b.iter(|| {
            black_box(42)
        })
    });

    group.finish();
}

fn bench_buffer_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_write");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    group.bench_function("write", |b| {
        b.iter(|| {
            black_box(42)
        })
    });

    group.finish();
}

fn bench_buffer_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_search");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    group.bench_function("search", |b| {
        b.iter(|| {
            black_box(42)
        })
    });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(10))
        .sample_size(50);
    targets = bench_terminal_creation, bench_buffer_write, bench_buffer_search
);
criterion_main!(benches);