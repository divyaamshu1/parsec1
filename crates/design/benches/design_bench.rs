#![allow(dead_code, unused_imports)]

//! Design benchmarks

use criterion::{criterion_group, criterion_main, Criterion, black_box};
use std::time::Duration;

fn bench_color_picker(c: &mut Criterion) {
    let mut group = c.benchmark_group("color_picker");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    group.bench_function("convert_color", |b| {
        b.iter(|| {
            black_box(42)
        })
    });

    group.finish();
}

fn bench_icon_browser(c: &mut Criterion) {
    let mut group = c.benchmark_group("icon_browser");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    group.bench_function("search_icons", |b| {
        b.iter(|| {
            black_box(42)
        })
    });

    group.finish();
}

fn bench_font_preview(c: &mut Criterion) {
    let mut group = c.benchmark_group("font_preview");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    group.bench_function("render_text", |b| {
        b.iter(|| {
            black_box(42)
        })
    });

    group.finish();
}

fn bench_svg_editor(c: &mut Criterion) {
    let mut group = c.benchmark_group("svg_editor");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    group.bench_function("parse_svg", |b| {
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
    targets = bench_color_picker, bench_icon_browser, bench_font_preview, bench_svg_editor
);
criterion_main!(benches);