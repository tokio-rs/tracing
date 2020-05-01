use criterion::{criterion_group, criterion_main, Criterion};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tempdir::TempDir;
use tracing::{event, Level};
use tracing_appender::non_blocking::NonBlocking;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::fmt::MakeWriter;

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

fn multi_threaded_bench(non_blocking: NonBlocking, iters: u64) -> Duration {
    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    let start = Instant::now();

    let cloned_make_writer = non_blocking;
    let cloned_make_writer_2 = cloned_make_writer.clone();

    handles.push(thread::spawn(move || {
        let subscriber = tracing_subscriber::fmt().with_writer(cloned_make_writer);
        tracing::subscriber::with_default(subscriber.finish(), || {
            for _ in 0..iters {
                event!(Level::INFO, "event");
            }
        });
    }));

    handles.push(thread::spawn(move || {
        let subscriber = tracing_subscriber::fmt().with_writer(cloned_make_writer_2);
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

            multi_threaded_bench(non_blocking, iters)
        });
    });
}

fn non_blocking_rolling_appender_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("non_blocking_rolling_file_appender");

    group.bench_function("non_blocking", |b| {
        let temp_dir = TempDir::new("rolling_file_appender").expect("Failed to create temp dir");
        let file_appender = rolling::hourly(temp_dir.path(), "log");
        let (non_blocking, _guard) = non_blocking(file_appender);

        let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);
        tracing::subscriber::with_default(subscriber.finish(), || {
            b.iter(|| event!(Level::INFO, "non_blocking event"))
        });

        let _ = temp_dir.close();
    });

    group.bench_function("multiple_writers", |b| {
        b.iter_custom(|iters| {
            let temp_dir =
                TempDir::new("rolling_file_appender").expect("Failed to create temp dir");
            let file_appender = rolling::hourly(temp_dir.path(), "log");
            let (non_blocking, _guard) = non_blocking(file_appender);

            multi_threaded_bench(non_blocking, iters)
        });
    });
}

criterion_group!(
    benches,
    non_blocking_benchmark,
    non_blocking_rolling_appender_benchmark
);
criterion_main!(benches);
