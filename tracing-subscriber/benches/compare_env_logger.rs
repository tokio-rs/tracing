use criterion::{criterion_group, criterion_main, Criterion};
mod my_module {
    pub(crate) fn tracing_enabled() {
        tracing::debug!("hello")
    }

    pub(crate) fn log_enabled() {
        log::debug!("hello")
    }
}

fn bench_event(c: &mut Criterion) {
    const FILTER: &str = concat!("info,", module_path!(), "::my_module=debug");
    let fmt = tracing_subscriber::fmt().with_env_filter(FILTER).finish();
    tracing::collect::set_global_default(fmt).expect("failed to set global subscriber");
    env_logger::builder().parse_filters(FILTER).init();

    {
        let mut enabled = c.benchmark_group("comparison/enabled");
        enabled.bench_function("tracing", |b| {
            b.iter(|| tracing::info!("hi"));
        });
        enabled.bench_function("log", |b| {
            b.iter(|| log::info!("hi"));
        });
    }

    {
        let mut enabled_target = c.benchmark_group("comparison/enabled_target");
        enabled_target.bench_function("tracing", |b| {
            b.iter(|| my_module::tracing_enabled());
        });
        enabled_target.bench_function("log", |b| {
            b.iter(|| my_module::log_enabled());
        });
    }
    {
        let mut max_level = c.benchmark_group("comparison/disabled_max_level");
        max_level.bench_function("tracing", |b| {
            b.iter(|| tracing::trace!("hi"));
        });
        max_level.bench_function("log", |b| {
            b.iter(|| log::trace!("hi"));
        });
    }
    {
        let mut disabled_target = c.benchmark_group("comparison/disabled_target");
        disabled_target.bench_function("tracing", |b| {
            b.iter(|| tracing::debug!("hi"));
        });
        disabled_target.bench_function("log", |b| {
            b.iter(|| log::debug!("hi"));
        });
    }
}

criterion_group!(benches, bench_event);
criterion_main!(benches);
