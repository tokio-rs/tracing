use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

mod shared;

fn bench(c: &mut Criterion) {
    shared::for_all_dispatches(&mut c.benchmark_group("Dispatch::get_ref"), |b| {
        b.iter(|| {
            tracing::dispatcher::get_default(|current| {
                black_box(&current);
            })
        })
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
