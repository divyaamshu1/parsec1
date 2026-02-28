#![allow(dead_code, unused_imports)]

//! Collaboration benchmarks

use criterion::{criterion_group, criterion_main, Criterion, black_box};
use std::time::Duration;

fn bench_live_share(c: &mut Criterion) {
    let mut group = c.benchmark_group("live_share");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    group.bench_function("broadcast_update", |b| {
        b.iter(|| {
            black_box(42)
        })
    });

    group.finish();
}

fn bench_comments(c: &mut Criterion) {
    let mut group = c.benchmark_group("comments");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    group.bench_function("add_comment", |b| {
        b.iter(|| {
            black_box(42)
        })
    });

    group.finish();
}

fn bench_crdt(c: &mut Criterion) {
    let mut group = c.benchmark_group("crdt");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    group.bench_function("merge_documents", |b| {
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
    targets = bench_live_share, bench_comments, bench_crdt
);
criterion_main!(benches);