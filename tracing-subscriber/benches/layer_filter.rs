use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::{
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};
use tracing::{dispatcher::Dispatch, span, Event, Id, Metadata, Subscriber};
use tracing_subscriber::{
    layer::{Context, Filter},
    prelude::*,
    registry::Registry,
    Layer,
};

mod support;
use support::MultithreadedBench;

struct NopLayer;

impl<S: Subscriber> Layer<S> for NopLayer {
    fn on_record(&self, span: &span::Id, values: &span::Record<'_>, ctx: Context<'_, S>) {
        // criterion::black_box((span, values, ctx));
    }

    fn on_follows_from(&self, span: &span::Id, follows: &span::Id, ctx: Context<'_, S>) {
        // criterion::black_box((span, follows, ctx));
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        // criterion::black_box((event, ctx));
    }

    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        // criterion::black_box((id, ctx));
    }

    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
        // criterion::black_box((id, ctx));
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        // criterion::black_box((id, ctx));
    }

    fn on_id_change(&self, old: &span::Id, new: &span::Id, ctx: Context<'_, S>) {
        // criterion::black_box((old, new, ctx));
    }
}
struct FakeFilter {
    enabled: bool,
}

impl<L, S> Filter<L, S> for FakeFilter
where
    L: Layer<S>,
    S: Subscriber,
{
    fn filter(&self, metadata: &Metadata<'_>, ctx: &Context<'_, S>) -> bool {
        criterion::black_box((metadata, ctx));
        self.enabled
    }
}

fn single_threaded(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_threaded");
    let mut bench_span = |n_layers: usize, dispatch: fn() -> tracing::Dispatch| {
        group.bench_with_input(BenchmarkId::new("span", n_layers), &n_layers, |b, _| {
            let _guard = tracing::dispatcher::set_default(&(dispatch)());
            b.iter(|| {
                let span = tracing::info_span!("test");
                let entered = span.enter();
                drop(entered);
                drop(span);
            })
        });
    };
    bench_span(0, || {
        let s = Registry::default().with(NopLayer).with(NopLayer);
        tracing::Dispatch::new(s)
    });

    bench_span(2, || {
        let s = Registry::default()
            .with(NopLayer.with_filter(FakeFilter { enabled: true }))
            .with(NopLayer.with_filter(FakeFilter { enabled: false }));
        tracing::Dispatch::new(s)
    });
    bench_span(4, || {
        let s = Registry::default()
            .with(NopLayer.with_filter(FakeFilter { enabled: true }))
            .with(NopLayer.with_filter(FakeFilter { enabled: false }))
            .with(NopLayer.with_filter(FakeFilter { enabled: true }))
            .with(NopLayer.with_filter(FakeFilter { enabled: false }));
        tracing::Dispatch::new(s)
    });
    bench_span(8, || {
        let s = Registry::default()
            .with(NopLayer.with_filter(FakeFilter { enabled: true }))
            .with(NopLayer.with_filter(FakeFilter { enabled: false }))
            .with(NopLayer.with_filter(FakeFilter { enabled: true }))
            .with(NopLayer.with_filter(FakeFilter { enabled: false }))
            .with(NopLayer.with_filter(FakeFilter { enabled: true }))
            .with(NopLayer.with_filter(FakeFilter { enabled: false }))
            .with(NopLayer.with_filter(FakeFilter { enabled: true }))
            .with(NopLayer.with_filter(FakeFilter { enabled: false }));
        tracing::Dispatch::new(s)
    });

    group.finish();
}

fn multi_threaded(c: &mut Criterion) {
    let mut group = c.benchmark_group("multithreaded");
    let mut bench_span = |n_layers: usize, dispatch: fn() -> tracing::Dispatch| {
        group.bench_with_input(BenchmarkId::new("span", n_layers), &n_layers, |b, _| {
            b.iter_custom(|iters| {
                let mut total = Duration::from_secs(0);
                for _ in 0..iters {
                    let bench = MultithreadedBench::new(dispatch());
                    let elapsed = bench
                        .thread(move || {
                            let span = tracing::info_span!("test");
                            let entered = span.enter();
                            drop(entered);
                            drop(span);
                        })
                        .thread(move || {
                            let span = tracing::info_span!("test");
                            let entered = span.enter();
                            drop(entered);
                            drop(span);
                        })
                        .thread(move || {
                            let span = tracing::info_span!("test");
                            let entered = span.enter();
                            drop(entered);
                            drop(span);
                        })
                        .thread(move || {
                            let span = tracing::info_span!("test");
                            let entered = span.enter();
                            drop(entered);
                            drop(span);
                        })
                        .run();
                    total += elapsed;
                }
                total
            })
        });
        group.bench_with_input(
            BenchmarkId::new("shared_span", n_layers),
            &n_layers,
            |b, _| {
                b.iter_custom(|iters| {
                    let mut total = Duration::from_secs(0);
                    for _ in 0..iters {
                        let dispatch = dispatch();
                        let bench = MultithreadedBench::new(dispatch.clone());
                        let span = tracing::dispatcher::with_default(&dispatch, || {
                            tracing::info_span!("test")
                        });
                        let span1 = span.clone();
                        let span2 = span.clone();
                        let span3 = span.clone();
                        let span4 = span.clone();
                        let span = tracing::info_span!("test");
                        let elapsed = bench
                            .thread(move || {
                                let entered = span1.enter();
                                drop(entered);
                            })
                            .thread(move || {
                                let entered = span2.enter();
                                drop(entered);
                            })
                            .thread(move || {
                                let entered = span3.enter();
                                drop(entered);
                            })
                            .thread(move || {
                                let entered = span4.enter();
                                drop(entered);
                            })
                            .run();
                        total += elapsed;
                    }
                    total
                })
            },
        );
    };
    bench_span(0, || {
        let s = Registry::default().with(NopLayer).with(NopLayer);
        tracing::Dispatch::new(s)
    });
    bench_span(2, || {
        let s = Registry::default()
            .with(NopLayer.with_filter(FakeFilter { enabled: true }))
            .with(NopLayer.with_filter(FakeFilter { enabled: false }));
        tracing::Dispatch::new(s)
    });
    bench_span(4, || {
        let s = Registry::default()
            .with(NopLayer.with_filter(FakeFilter { enabled: true }))
            .with(NopLayer.with_filter(FakeFilter { enabled: false }))
            .with(NopLayer.with_filter(FakeFilter { enabled: true }))
            .with(NopLayer.with_filter(FakeFilter { enabled: false }));
        tracing::Dispatch::new(s)
    });
    bench_span(8, || {
        let s = Registry::default()
            .with(NopLayer.with_filter(FakeFilter { enabled: true }))
            .with(NopLayer.with_filter(FakeFilter { enabled: false }))
            .with(NopLayer.with_filter(FakeFilter { enabled: true }))
            .with(NopLayer.with_filter(FakeFilter { enabled: false }))
            .with(NopLayer.with_filter(FakeFilter { enabled: true }))
            .with(NopLayer.with_filter(FakeFilter { enabled: false }))
            .with(NopLayer.with_filter(FakeFilter { enabled: true }))
            .with(NopLayer.with_filter(FakeFilter { enabled: false }));
        tracing::Dispatch::new(s)
    });

    group.finish();
}

criterion_group!(benches, single_threaded, multi_threaded);
criterion_main!(benches);
