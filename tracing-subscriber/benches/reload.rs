use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rayon::prelude::*;
use rayon::ThreadPool;
use rayon::ThreadPoolBuilder;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tracing::{dispatcher::Dispatch, span, Event, Id, Metadata};
use tracing_subscriber::{filter::LevelFilter, prelude::*, reload};

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

#[inline]
fn emit_workload() {
    tracing::info!("hello");
    tracing::debug!("hello");
    tracing::trace!("hello");
}

fn run_threads(dispatch: Dispatch, threads: usize, ops_per_thread: usize) -> Duration {
    let start = Arc::new(Barrier::new(threads + 1));
    let end = Arc::new(Barrier::new(threads + 1));

    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let dispatch = dispatch.clone();
            let start = start.clone();
            let end = end.clone();
            thread::spawn(move || {
                tracing::dispatcher::with_default(&dispatch, || {
                    start.wait();
                    for _ in 0..ops_per_thread {
                        emit_workload();
                    }
                    end.wait();
                })
            })
        })
        .collect();

    start.wait();
    let t0 = Instant::now();
    end.wait();
    let elapsed = t0.elapsed();

    for h in handles {
        h.join().expect("worker thread panicked");
    }

    elapsed
}

fn run_tokio_spawn_blocking(
    rt: &Runtime,
    dispatch: Dispatch,
    tasks: usize,
    ops_per_task: usize,
) -> Duration {
    let start = Arc::new(Barrier::new(tasks + 1));
    let end = Arc::new(Barrier::new(tasks + 1));

    let handle = rt.handle().clone();
    let mut joins = Vec::with_capacity(tasks);

    for _ in 0..tasks {
        let dispatch = dispatch.clone();
        let start = start.clone();
        let end = end.clone();
        joins.push(handle.spawn_blocking(move || {
            tracing::dispatcher::with_default(&dispatch, || {
                start.wait();
                for _ in 0..ops_per_task {
                    emit_workload();
                }
                end.wait();
            })
        }));
    }

    start.wait();
    let t0 = Instant::now();
    end.wait();
    let elapsed = t0.elapsed();

    rt.block_on(async {
        for j in joins {
            j.await.expect("spawn_blocking task panicked");
        }
    });

    elapsed
}

fn run_rayon(pool: &ThreadPool, dispatch: Dispatch, tasks: usize, ops_per_task: usize) -> Duration {
    pool.install(|| {
        let t0 = Instant::now();
        (0..tasks).into_par_iter().for_each(|_| {
            let dispatch = dispatch.clone();
            tracing::dispatcher::with_default(&dispatch, || {
                for _ in 0..ops_per_task {
                    emit_workload();
                }
            });
        });
        t0.elapsed()
    })
}

fn reload_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("reload");

    group.bench_function(BenchmarkId::new("single_threaded", "1_no_reload"), |b| {
        let subscriber = EnabledSubscriber.with(LevelFilter::INFO);
        tracing::subscriber::with_default(subscriber, || b.iter(emit_workload));
    });

    group.bench_function(BenchmarkId::new("single_threaded", "2_rwlock"), |b| {
        let (layer, _handle) = reload::Layer::new(LevelFilter::INFO);
        let subscriber = EnabledSubscriber.with(layer);
        tracing::subscriber::with_default(subscriber, || b.iter(emit_workload));
    });

    #[cfg(feature = "reload-arc-swap")]
    group.bench_function(BenchmarkId::new("single_threaded", "3_arc_swap"), |b| {
        let (layer, _handle) = reload::ArcSwapLayer::new(LevelFilter::INFO);
        let subscriber = EnabledSubscriber.with(layer);
        tracing::subscriber::with_default(subscriber, || b.iter(emit_workload));
    });

    group.bench_function(
        BenchmarkId::new("multithreaded_16x1000", "1_no_reload"),
        |b| {
            let dispatch = Dispatch::new(EnabledSubscriber.with(LevelFilter::INFO));
            b.iter_custom(|iters| {
                let mut total = Duration::from_secs(0);
                for _ in 0..iters {
                    total += run_threads(dispatch.clone(), 16, 1000);
                }
                total
            });
        },
    );

    group.bench_function(BenchmarkId::new("multithreaded_16x1000", "2_rwlock"), |b| {
        let (layer, _handle) = reload::Layer::new(LevelFilter::INFO);
        let dispatch = Dispatch::new(EnabledSubscriber.with(layer));
        b.iter_custom(|iters| {
            let mut total = Duration::from_secs(0);
            for _ in 0..iters {
                total += run_threads(dispatch.clone(), 16, 1000);
            }
            total
        });
    });

    #[cfg(feature = "reload-arc-swap")]
    group.bench_function(
        BenchmarkId::new("multithreaded_16x1000", "3_arc_swap"),
        |b| {
            let (layer, _handle) = reload::ArcSwapLayer::new(LevelFilter::INFO);
            let dispatch = Dispatch::new(EnabledSubscriber.with(layer));
            b.iter_custom(|iters| {
                let mut total = Duration::from_secs(0);
                for _ in 0..iters {
                    total += run_threads(dispatch.clone(), 16, 1000);
                }
                total
            });
        },
    );

    // These benchmarks intentionally use APIs that produce OS-thread parallelism
    // (rather than typical Tokio runtime worker-thread multiplexing), to stress
    // `reload::Layer`'s read-side synchronization overhead.
    group.bench_function(
        BenchmarkId::new("tokio_spawn_blocking_16x1000", "1_no_reload"),
        |b| {
            let rt = Runtime::new().expect("tokio runtime");
            let dispatch = Dispatch::new(EnabledSubscriber.with(LevelFilter::INFO));
            b.iter_custom(|iters| {
                let mut total = Duration::from_secs(0);
                for _ in 0..iters {
                    total += run_tokio_spawn_blocking(&rt, dispatch.clone(), 16, 1000);
                }
                total
            });
        },
    );

    group.bench_function(
        BenchmarkId::new("tokio_spawn_blocking_16x1000", "2_rwlock"),
        |b| {
            let rt = Runtime::new().expect("tokio runtime");
            let (layer, _handle) = reload::Layer::new(LevelFilter::INFO);
            let dispatch = Dispatch::new(EnabledSubscriber.with(layer));
            b.iter_custom(|iters| {
                let mut total = Duration::from_secs(0);
                for _ in 0..iters {
                    total += run_tokio_spawn_blocking(&rt, dispatch.clone(), 16, 1000);
                }
                total
            });
        },
    );

    #[cfg(feature = "reload-arc-swap")]
    group.bench_function(
        BenchmarkId::new("tokio_spawn_blocking_16x1000", "3_arc_swap"),
        |b| {
            let rt = Runtime::new().expect("tokio runtime");
            let (layer, _handle) = reload::ArcSwapLayer::new(LevelFilter::INFO);
            let dispatch = Dispatch::new(EnabledSubscriber.with(layer));
            b.iter_custom(|iters| {
                let mut total = Duration::from_secs(0);
                for _ in 0..iters {
                    total += run_tokio_spawn_blocking(&rt, dispatch.clone(), 16, 1000);
                }
                total
            });
        },
    );

    group.bench_function(BenchmarkId::new("rayon_16x1000", "1_no_reload"), |b| {
        let pool = ThreadPoolBuilder::new()
            .num_threads(16)
            .build()
            .expect("rayon thread pool");
        let dispatch = Dispatch::new(EnabledSubscriber.with(LevelFilter::INFO));
        b.iter_custom(|iters| {
            let mut total = Duration::from_secs(0);
            for _ in 0..iters {
                total += run_rayon(&pool, dispatch.clone(), 16, 1000);
            }
            total
        });
    });

    group.bench_function(BenchmarkId::new("rayon_16x1000", "2_rwlock"), |b| {
        let pool = ThreadPoolBuilder::new()
            .num_threads(16)
            .build()
            .expect("rayon thread pool");
        let (layer, _handle) = reload::Layer::new(LevelFilter::INFO);
        let dispatch = Dispatch::new(EnabledSubscriber.with(layer));
        b.iter_custom(|iters| {
            let mut total = Duration::from_secs(0);
            for _ in 0..iters {
                total += run_rayon(&pool, dispatch.clone(), 16, 1000);
            }
            total
        });
    });

    #[cfg(feature = "reload-arc-swap")]
    group.bench_function(BenchmarkId::new("rayon_16x1000", "3_arc_swap"), |b| {
        let pool = ThreadPoolBuilder::new()
            .num_threads(16)
            .build()
            .expect("rayon thread pool");
        let (layer, _handle) = reload::ArcSwapLayer::new(LevelFilter::INFO);
        let dispatch = Dispatch::new(EnabledSubscriber.with(layer));
        b.iter_custom(|iters| {
            let mut total = Duration::from_secs(0);
            for _ in 0..iters {
                total += run_rayon(&pool, dispatch.clone(), 16, 1000);
            }
            total
        });
    });

    group.finish();
}

criterion_group!(benches, reload_benchmark);
criterion_main!(benches);
