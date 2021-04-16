use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("facade");
    group.bench_function("log", |b| {
        b.iter(|| {
            log::info!("Rust empowering everyone to build reliable and efficient software.");
        });
    });

    group.bench_function("tracing", |b| {
        b.iter(|| {
            tracing::info!("Rust empowering everyone to build reliable and efficient software.");
        });
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
