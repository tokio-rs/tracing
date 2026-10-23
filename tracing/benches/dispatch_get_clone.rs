use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

mod shared;

fn bench(c: &mut Criterion) {
    shared::for_all_dispatches(&mut c.benchmark_group("Dispatch::get_clone"), |b| {
        b.iter(|| {
            let current = tracing::dispatcher::get_default(|current| current.clone());
            black_box(current);
        })
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
