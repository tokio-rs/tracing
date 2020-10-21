use criterion::{criterion_group, criterion_main, Criterion};
use tracing_subscriber::prelude::*;

fn enter(c: &mut Criterion) {
    let mut group = c.benchmark_group("enter");
    let _subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .finish()
        .set_default();
    group.bench_function("enabled", |b| {
        let span = tracing::info_span!("foo");
        b.iter_with_large_drop(|| span.enter())
    });
    group.bench_function("disabled", |b| {
        let span = tracing::debug_span!("foo");
        b.iter_with_large_drop(|| span.enter())
    });
}

fn enter_exit(c: &mut Criterion) {
    let mut group = c.benchmark_group("enter_exit");
    let _subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .finish()
        .set_default();
    group.bench_function("enabled", |b| {
        let span = tracing::info_span!("foo");
        b.iter(|| span.enter())
    });
    group.bench_function("disabled", |b| {
        let span = tracing::debug_span!("foo");
        b.iter(|| span.enter())
    });
}

criterion_group!(benches, enter, enter_exit);
criterion_main!(benches);
