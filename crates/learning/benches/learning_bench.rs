#![allow(dead_code)]

//! Learning benchmarks

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, black_box};
use std::time::Duration;

fn bench_tutorial_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("tutorial_loading");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    group.bench_function("load_tutorials", |b| {
        b.iter(|| {
            black_box(42)
        })
    });

    group.finish();
}

fn bench_playground_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("playground_execution");
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(30);

    let languages = ["rust", "python", "javascript"];

    for lang in languages.iter() {
        group.bench_with_input(BenchmarkId::new("run", lang), lang, |b, _| {
            b.iter(|| {
                black_box(lang)
            })
        });
    }

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(10))
        .sample_size(100);
    targets = bench_tutorial_loading, bench_playground_execution
);
criterion_main!(benches);