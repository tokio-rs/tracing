use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;
use tracing::Dispatch;
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{
        format::{DefaultFields, Format, Full},
        time::ChronoLocal,
        Collector, CollectorBuilder,
    },
    EnvFilter,
};

mod support;
use support::{MultithreadedBench, NoWriter};

/// Prepare a real-world-inspired collector and use it to  measure the performance impact of
/// reloadable collectors.

fn mk_builder(
) -> CollectorBuilder<DefaultFields, Format<Full, ChronoLocal>, EnvFilter, fn() -> NoWriter> {
    let filter = EnvFilter::default()
        .add_directive("this=off".parse().unwrap())
        .add_directive("also=off".parse().unwrap())
        .add_directive("this_too=off".parse().unwrap())
        .add_directive("stuff=warn".parse().unwrap())
        .add_directive("moar=warn".parse().unwrap())
        .add_directive(LevelFilter::INFO.into());

    Collector::builder()
        .with_env_filter(filter)
        .with_writer(NoWriter::new as _)
        .with_timer(ChronoLocal::with_format(
            "%Y-%m-%d %H:%M:%S%.3f".to_string(),
        ))
        .with_ansi(true)
        .with_target(true)
        .with_level(true)
        .with_thread_names(true)
}

fn bench_reload(c: &mut Criterion) {
    let mut group = c.benchmark_group("reload");
    group.bench_function("complex-collector", |b| {
        let builder = mk_builder();
        let collector = builder.finish();
        tracing::collect::with_default(collector, || {
            b.iter(|| {
                tracing::info!(target: "this", "hi");
                tracing::debug!(target: "enabled", "hi");
                tracing::warn!(target: "hepp", "hi");
                tracing::trace!(target: "stuff", "hi");
            })
        });
    });
    group.bench_function("complex-collector-reloadable", |b| {
        let builder = mk_builder().with_filter_reloading();
        let _handle = builder.reload_handle();
        let collector = builder.finish();
        tracing::collect::with_default(collector, || {
            b.iter(|| {
                tracing::info!(target: "this", "hi");
                tracing::debug!(target: "enabled", "hi");
                tracing::warn!(target: "hepp", "hi");
                tracing::trace!(target: "stuff", "hi");
            })
        });
    });
    group.finish();
}

fn bench_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("reload_concurrent");

    fn events() {
        tracing::info!(target: "this", "hi");
        tracing::debug!(target: "enabled", "hi");
        tracing::warn!(target: "hepp", "hi");
        tracing::trace!(target: "stuff", "hi");
    }

    group.bench_function("complex-collector", |b| {
        let builder = mk_builder();
        let collector = Dispatch::new(builder.finish());
        b.iter_custom(|iters| {
            let mut total = Duration::from_secs(0);
            for _ in 0..iters {
                let bench = MultithreadedBench::new(collector.clone());
                let elapsed = bench
                    .thread(events)
                    .thread(events)
                    .thread(events)
                    .thread(events)
                    .run();
                total += elapsed;
            }
            total
        });
    });
    group.bench_function("complex-collector-reloadable", |b| {
        let builder = mk_builder().with_filter_reloading();
        let _handle = builder.reload_handle();
        let collector = Dispatch::new(builder.finish());
        b.iter_custom(|iters| {
            let mut total = Duration::from_secs(0);
            for _ in 0..iters {
                let bench = MultithreadedBench::new(collector.clone());
                let elapsed = bench
                    .thread(events)
                    .thread(events)
                    .thread(events)
                    .thread(events)
                    .run();
                total += elapsed;
            }
            total
        });
    });
    group.finish();
}

criterion_group!(benches, bench_reload, bench_concurrent);
criterion_main!(benches);
