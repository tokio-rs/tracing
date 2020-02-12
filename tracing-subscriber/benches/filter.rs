use criterion::{criterion_group, criterion_main, Criterion};
use std::{
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};
use tracing::{dispatcher::Dispatch, span, Event, Id, Metadata};
use tracing_subscriber::{prelude::*, EnvFilter};

/// A subscriber that is enabled but otherwise does nothing.
struct EnabledSubscriber;

impl tracing::Subscriber for EnabledSubscriber {
    fn new_span(&self, span: &span::Attributes<'_>) -> Id {
        let _ = span;
        Id::from_u64(0xDEAD_FACE)
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

#[derive(Clone)]
struct MultithreadedBench {
    start: Arc<Barrier>,
    end: Arc<Barrier>,
    dispatch: Dispatch,
}

impl MultithreadedBench {
    fn new(dispatch: Dispatch) -> Self {
        Self {
            start: Arc::new(Barrier::new(5)),
            end: Arc::new(Barrier::new(5)),
            dispatch,
        }
    }

    fn thread(&self, f: impl FnOnce(&Barrier) + Send + 'static) -> &Self {
        let this = self.clone();
        thread::spawn(move || {
            let dispatch = this.dispatch.clone();
            tracing::dispatcher::with_default(&dispatch, move || {
                f(&*this.start);
                this.end.wait();
            })
        });
        self
    }

    fn run(&self) -> Duration {
        self.start.wait();
        let t0 = Instant::now();
        self.end.wait();
        t0.elapsed()
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
    group.bench_function("enabled_one", |b| {
        let filter = "static_filter=info"
            .parse::<EnvFilter>()
            .expect("should parse");
        tracing::subscriber::with_default(EnabledSubscriber.with(filter), || {
            b.iter(|| {
                tracing::info!(target: "static_filter", "hi");
            })
        });
    });
    group.bench_function("enabled_many", |b| {
        let filter = "foo=debug,bar=trace,baz=error,quux=warn,static_filter=info"
            .parse::<EnvFilter>()
            .expect("should parse");
        tracing::subscriber::with_default(EnabledSubscriber.with(filter), || {
            b.iter(|| {
                tracing::info!(target: "static_filter", "hi");
            })
        });
    });
    group.bench_function("disabled_level_one", |b| {
        let filter = "static_filter=info"
            .parse::<EnvFilter>()
            .expect("should parse");
        tracing::subscriber::with_default(EnabledSubscriber.with(filter), || {
            b.iter(|| {
                tracing::debug!(target: "static_filter", "hi");
            })
        });
    });
    group.bench_function("disabled_level_many", |b| {
        let filter = "foo=debug,bar=info,baz=error,quux=warn,static_filter=info"
            .parse::<EnvFilter>()
            .expect("should parse");
        tracing::subscriber::with_default(EnabledSubscriber.with(filter), || {
            b.iter(|| {
                tracing::trace!(target: "static_filter", "hi");
            })
        });
    });
    group.bench_function("disabled_one", |b| {
        let filter = "foo=info"
            .parse::<EnvFilter>()
            .expect("should parse");
        tracing::subscriber::with_default(EnabledSubscriber.with(filter), || {
            b.iter(|| {
                tracing::info!(target: "static_filter", "hi");
            })
        });
    });
    group.bench_function("disabled_many", |b| {
        let filter = "foo=debug,bar=trace,baz=error,quux=warn,whibble=info"
            .parse::<EnvFilter>()
            .expect("should parse");
        tracing::subscriber::with_default(EnabledSubscriber.with(filter), || {
            b.iter(|| {
                tracing::info!(target: "static_filter", "hi");
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
                    .thread(|start| {
                        start.wait();
                        tracing::info!(target: "static_filter", "hi");
                    })
                    .thread(|start| {
                        start.wait();
                        tracing::debug!(target: "static_filter", "hi");
                    })
                    .thread(|start| {
                        start.wait();
                        tracing::warn!(target: "static_filter", "hi");
                    })
                    .thread(|start| {
                        start.wait();
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
                    .thread(|start| {
                        start.wait();
                        tracing::info!(target: "static_filter", "hi");
                    })
                    .thread(|start| {
                        start.wait();
                        tracing::debug!(target: "static_filter", "hi");
                    })
                    .thread(|start| {
                        start.wait();
                        tracing::warn!(target: "static_filter", "hi");
                    })
                    .thread(|start| {
                        start.wait();
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
                    .thread(|start| {
                        let span = tracing::info_span!("foo");
                        start.wait();
                        let _ = span.enter();
                        tracing::info!("hi");
                    })
                    .thread(|start| {
                        let span = tracing::info_span!("foo");
                        start.wait();
                        let _ = span.enter();
                        tracing::debug!("hi");
                    })
                    .thread(|start| {
                        let span = tracing::info_span!("bar");
                        start.wait();
                        let _ = span.enter();
                        tracing::debug!("hi");
                    })
                    .thread(|start| {
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
                    .thread(|start| {
                        let span = tracing::info_span!("foo");
                        start.wait();
                        let _ = span.enter();
                        tracing::info!("hi");
                    })
                    .thread(|start| {
                        let span = tracing::info_span!("foo");
                        start.wait();
                        let _ = span.enter();
                        tracing::debug!("hi");
                    })
                    .thread(|start| {
                        let span = tracing::info_span!("bar");
                        start.wait();
                        let _ = span.enter();
                        tracing::debug!("hi");
                    })
                    .thread(|start| {
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

fn bench_mixed(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed");
    group.bench_function("disabled", |b| {
        let filter = "[foo]=trace,bar[quux]=debug,[{baz}]=debug,asdf=warn,wibble=info"
            .parse::<EnvFilter>()
            .expect("should parse");
        tracing::subscriber::with_default(EnabledSubscriber.with(filter), || {
            b.iter(|| {
                tracing::info!(target: "static_filter", "hi");
            })
        });
    });
    group.bench_function("disabled_by_level", |b| {
        let filter = "[foo]=info,bar[quux]=debug,asdf=warn,static_filter=info"
            .parse::<EnvFilter>()
            .expect("should parse");
        tracing::subscriber::with_default(EnabledSubscriber.with(filter), || {
            b.iter(|| {
                tracing::trace!(target: "static_filter", "hi");
            })
        });
    });
}

criterion_group!(benches, bench_static, bench_dynamic, bench_mixed);
criterion_main!(benches);
