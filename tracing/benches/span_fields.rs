use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use tracing::{span, Level};

mod shared;

fn bench(c: &mut Criterion) {
    shared::for_all_recording(&mut c.benchmark_group("span_fields"), |b| {
        b.iter(|| {
            let span = span!(
                Level::TRACE,
                "span",
                foo = "foo",
                bar = "bar",
                baz = 3,
                quuux = tracing::field::debug(0.99)
            );
            black_box(span)
        })
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
