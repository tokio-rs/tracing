use criterion::{criterion_group, criterion_main, Criterion};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;
use tracing::{event, Level};
use tracing_appender::non_blocking;
use tracing_subscriber::fmt::MakeWriter;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct SynchronousWriter {
    writer: Arc<Mutex<Vec<u8>>>,
}

impl SynchronousWriter {
    fn new() -> SynchronousWriter {
        SynchronousWriter {
            writer: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl MakeWriter for SynchronousWriter {
    type Writer = SynchronousWriter;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}

impl std::io::Write for SynchronousWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let buf_len = buf.len();
        match self.writer.lock() {
            Ok(mut guard) => {
                guard.extend_from_slice(buf);
            },
            Err(e) => {
                eprintln!("Failed to acquire lock: {:?}", e);
            },
        }
        Ok(buf_len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A cheap writer so that we don't spam console if we had used stdout
#[derive(Clone)]
struct SilentWriter {
    tx: Sender<String>,
}

impl SilentWriter {
    fn new() -> (Self, Receiver<String>) {
        let (tx, rx) = unbounded();
        (Self { tx }, rx)
    }
}

impl std::io::Write for SilentWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let buf_len = buf.len();
        let _ = self.tx.send(String::from_utf8_lossy(buf).to_string());
        Ok(buf_len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl MakeWriter for SilentWriter {
    type Writer = SilentWriter;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}

fn synchronous_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("synchronous");
    group.bench_function("single_thread", |b| {
        let subscriber = tracing_subscriber::fmt().with_writer(SynchronousWriter::new());
        tracing::subscriber::with_default(subscriber.finish(), || {
            b.iter(|| event!(Level::INFO, "event"))
        });
    });

    group.bench_function("multiple_writers", |b| {
        b.iter_custom(|iters| {
            let mut handles: Vec<JoinHandle<()>> = Vec::new();

            let start = Instant::now();

            let make_writer = SynchronousWriter::new();
            let cloned_make_writer = make_writer.clone();

            handles.push(thread::spawn(move || {
                let subscriber = tracing_subscriber::fmt().with_writer(make_writer);
                tracing::subscriber::with_default(subscriber.finish(), || {
                    for _ in 0..iters {
                        event!(Level::INFO, "event");
                    }
                });
            }));

            handles.push(thread::spawn(move || {
                let subscriber = tracing_subscriber::fmt().with_writer(cloned_make_writer);
                tracing::subscriber::with_default(subscriber.finish(), || {
                    for _ in 0..iters {
                        event!(Level::INFO, "event");
                    }
                });
            }));

            for handle in handles {
                let _ = handle.join();
            }

            start.elapsed()
        });
    });
}

fn non_blocking_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("non_blocking");

    group.bench_function("single_thread", |b| {
        let (silent_writer, _rx) = SilentWriter::new();
        let (non_blocking, _guard) = non_blocking(silent_writer);
        let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);

        tracing::subscriber::with_default(subscriber.finish(), || {
            b.iter(|| event!(Level::INFO, "event"))
        });
    });

    group.bench_function("multiple_writers", |b| {
        b.iter_custom(|iters| {
            let (silent_writer, _rx) = SilentWriter::new();
            let (non_blocking, _guard) = non_blocking(silent_writer);

            let mut handles: Vec<JoinHandle<()>> = Vec::new();

            let start = Instant::now();

            let cloned_make_writer = non_blocking.clone();

            handles.push(thread::spawn(move || {
                let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);
                tracing::subscriber::with_default(subscriber.finish(), || {
                    for _ in 0..iters {
                        event!(Level::INFO, "event");
                    }
                });
            }));

            handles.push(thread::spawn(move || {
                let subscriber = tracing_subscriber::fmt().with_writer(cloned_make_writer);
                tracing::subscriber::with_default(subscriber.finish(), || {
                    for _ in 0..iters {
                        event!(Level::INFO, "event");
                    }
                });
            }));

            for handle in handles {
                let _ = handle.join();
            }

            start.elapsed()
        });
    });
}

criterion_group!(
    benches,
    synchronous_benchmark,
    non_blocking_benchmark
);
criterion_main!(benches);
