use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tracing::{span, Collect};
use tracing_subscriber::{
    prelude::*,
    registry::LookupSpan,
    subscribe::{Context, Subscribe},
};

struct MySubscriber;

impl<C: Collect + for<'a> LookupSpan<'a>> Subscribe<C> for MySubscriber {
    fn on_enter(&self, id: &span::Id, cx: Context<'_, C>) {
        for span in cx.span_scope(id).unwrap() {
            black_box(span);
        }
    }
}

fn iter_scope(c: &mut Criterion) {
    const LENS: &[usize] = &[5, 10, 100, 1000];
    let mut group = c.benchmark_group("iter_scope");
    let _subscriber = tracing_subscriber::registry()
        .with(MySubscriber)
        .set_default();
    for &len in LENS {
        let mut curr_span = tracing::info_span!("root");
        for depth in 0..len {
            curr_span = tracing::info_span!(parent: curr_span.id(), "child", depth)
        }
        group.bench_with_input(BenchmarkId::new("span_scope", len), &len, |b, _| {
            b.iter(|| {
                let _e = curr_span.enter();
            });
        });
    }
}

criterion_group!(benches, iter_scope);
criterion_main!(benches);
