use criterion::{criterion_group, criterion_main, Criterion};
use std::{
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};
use tracing::{dispatcher::Dispatch, span, Event, Id, Metadata};
use tracing_subscriber::{prelude::*, EnvFilter};

mod support;
use support::MultithreadedBench;

/// A subscriber that is enabled but otherwise does nothing.
struct EnabledSubscriber;

impl tracing::Subscriber for EnabledSubscriber {
    fn new_span(&self, span: &span::Attributes<'_>) -> Id {
        let _ = span;
        Id::from_u64(0xDEADFACE)
    }

    fn event(&self, event: &Event<'_>) {
        let _ = event;
    }

    fn record(&self, span: &Id, values: &span::Record<'_>) {
        let _ = (span, values);
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        let _ = (span, follows);
    }

    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        let _ = metadata;
        true
    }

    fn enter(&self, span: &Id) {
        let _ = span;
    }

    fn exit(&self, span: &Id) {
        let _ = span;
    }
}

fn bench_static(c: &mut Criterion) {
    let mut group = c.benchmark_group("static");

    group.bench_function("baseline_single_threaded", |b| {
        tracing::subscriber::with_default(EnabledSubscriber, || {
            b.iter(|| {
                tracing::info!(target: "static_filter", "hi");
                tracing::debug!(target: "static_filter", "hi");
                tracing::warn!(target: "static_filter", "hi");
                tracing::trace!(target: "foo", "hi");
            })
        });
    });
    group.bench_function("single_threaded", |b| {
        let filter = "static_filter=info"
            .parse::<EnvFilter>()
            .expect("should parse");
        tracing::subscriber::with_default(EnabledSubscriber.with(filter), || {
            b.iter(|| {
                tracing::info!(target: "static_filter", "hi");
                tracing::debug!(target: "static_filter", "hi");
                tracing::warn!(target: "static_filter", "hi");
                tracing::trace!(target: "foo", "hi");
            })
        });
    });
    group.bench_function("baseline_multithreaded", |b| {
        let dispatch = tracing::dispatcher::Dispatch::new(EnabledSubscriber);
        b.iter_custom(|iters| {
            let mut total = Duration::from_secs(0);
            for _ in 0..iters {
                let bench = MultithreadedBench::new(dispatch.clone());
                let elapsed = bench
                    .thread(|| {
                        tracing::info!(target: "static_filter", "hi");
                    })
                    .thread(|| {
                        tracing::debug!(target: "static_filter", "hi");
                    })
                    .thread(|| {
                        tracing::warn!(target: "static_filter", "hi");
                    })
                    .thread(|| {
                        tracing::warn!(target: "foo", "hi");
                    })
                    .run();
                total += elapsed;
            }
            total
        })
    });
    group.bench_function("multithreaded", |b| {
        let filter = "static_filter=info"
            .parse::<EnvFilter>()
            .expect("should parse");
        let dispatch = tracing::dispatcher::Dispatch::new(EnabledSubscriber.with(filter));
        b.iter_custom(|iters| {
            let mut total = Duration::from_secs(0);
            for _ in 0..iters {
                let bench = MultithreadedBench::new(dispatch.clone());
                let elapsed = bench
                    .thread(|| {
                        tracing::info!(target: "static_filter", "hi");
                    })
                    .thread(|| {
                        tracing::debug!(target: "static_filter", "hi");
                    })
                    .thread(|| {
                        tracing::warn!(target: "static_filter", "hi");
                    })
                    .thread(|| {
                        tracing::warn!(target: "foo", "hi");
                    })
                    .run();
                total += elapsed;
            }
            total
        });
    });
    group.finish();
}

fn bench_dynamic(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic");

    group.bench_function("baseline_single_threaded", |b| {
        tracing::subscriber::with_default(EnabledSubscriber, || {
            b.iter(|| {
                tracing::info_span!("foo").in_scope(|| {
                    tracing::info!("hi");
                    tracing::debug!("hi");
                });
                tracing::info_span!("bar").in_scope(|| {
                    tracing::warn!("hi");
                });
                tracing::trace!("hi");
            })
        });
    });
    group.bench_function("single_threaded", |b| {
        let filter = "[foo]=trace".parse::<EnvFilter>().expect("should parse");
        tracing::subscriber::with_default(EnabledSubscriber.with(filter), || {
            b.iter(|| {
                tracing::info_span!("foo").in_scope(|| {
                    tracing::info!("hi");
                    tracing::debug!("hi");
                });
                tracing::info_span!("bar").in_scope(|| {
                    tracing::warn!("hi");
                });
                tracing::trace!("hi");
            })
        });
    });
    group.bench_function("baseline_multithreaded", |b| {
        let dispatch = tracing::dispatcher::Dispatch::new(EnabledSubscriber);
        b.iter_custom(|iters| {
            let mut total = Duration::from_secs(0);
            for _ in 0..iters {
                let bench = MultithreadedBench::new(dispatch.clone());
                let elapsed = bench
                    .thread_with_setup(|start| {
                        let span = tracing::info_span!("foo");
                        start.wait();
                        let _ = span.enter();
                        tracing::info!("hi");
                    })
                    .thread_with_setup(|start| {
                        let span = tracing::info_span!("foo");
                        start.wait();
                        let _ = span.enter();
                        tracing::debug!("hi");
                    })
                    .thread_with_setup(|start| {
                        let span = tracing::info_span!("bar");
                        start.wait();
                        let _ = span.enter();
                        tracing::debug!("hi");
                    })
                    .thread_with_setup(|start| {
                        start.wait();
                        tracing::trace!("hi");
                    })
                    .run();
                total += elapsed;
            }
            total
        })
    });
    group.bench_function("multithreaded", |b| {
        let filter = "[foo]=trace".parse::<EnvFilter>().expect("should parse");
        let dispatch = tracing::dispatcher::Dispatch::new(EnabledSubscriber.with(filter));
        b.iter_custom(|iters| {
            let mut total = Duration::from_secs(0);
            for _ in 0..iters {
                let bench = MultithreadedBench::new(dispatch.clone());
                let elapsed = bench
                    .thread_with_setup(|start| {
                        let span = tracing::info_span!("foo");
                        start.wait();
                        let _ = span.enter();
                        tracing::info!("hi");
                    })
                    .thread_with_setup(|start| {
                        let span = tracing::info_span!("foo");
                        start.wait();
                        let _ = span.enter();
                        tracing::debug!("hi");
                    })
                    .thread_with_setup(|start| {
                        let span = tracing::info_span!("bar");
                        start.wait();
                        let _ = span.enter();
                        tracing::debug!("hi");
                    })
                    .thread_with_setup(|start| {
                        start.wait();
                        tracing::trace!("hi");
                    })
                    .run();
                total += elapsed;
            }
            total
        })
    });

    group.finish();
}

criterion_group!(benches, bench_static, bench_dynamic);
criterion_main!(benches);
