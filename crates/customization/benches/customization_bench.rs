#![allow(dead_code, unused_imports)]

//! Customization benchmarks

use criterion::{criterion_group, criterion_main, Criterion, black_box};
use std::time::Duration;

fn bench_keybindings(c: &mut Criterion) {
    let mut group = c.benchmark_group("keybindings");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    group.bench_function("parse_keybindings", |b| {
        b.iter(|| {
            black_box(42)
        })
    });

    group.finish();
}

fn bench_themes(c: &mut Criterion) {
    let mut group = c.benchmark_group("themes");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    group.bench_function("apply_theme", |b| {
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
    targets = bench_keybindings, bench_themes
);
criterion_main!(benches);