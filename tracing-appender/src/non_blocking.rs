use crate::worker::Worker;
use crossbeam_channel::{bounded, Sender};
use std::io;
use std::io::Write;
use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;
use std::thread::JoinHandle;
use tracing_subscriber::fmt::MakeWriter;

/// The default maximum number of buffered log lines.
///
/// If [`NonBlocking`][non-blocking] is lossy, it will drop spans/events at capacity.
/// capacity. If [`NonBlocking`][non-blocking] is _not_ lossy,
/// backpressure will be exerted on senders, causing them to block their
/// respective threads until there is available capacity.
///
/// [non-blocking]: ./struct.NonBlocking.html
/// Recommended to be a power of 2.
pub const DEFAULT_BUFFERED_LINES_LIMIT: usize = 128_000;

/// A guard that flushes spans/events associated to a [`NonBlocking`] on a drop
///
/// Writing to a [`NonBlocking`] writer will **not** immediately write a span or event to the underlying
/// output. Instead, the span or event will be written by a dedicated logging thread at some later point.
/// To increase throughput, the non-blocking writer will flush to the underlying output on
/// a periodic basis rather than every time a span or event is written. This means that if the program
/// terminates abruptly (such as through an uncaught `panic` or a `std::process::exit`), some spans
/// or events may not be written.
///
/// [`NonBlocking`]: ./struct.NonBlocking.html
/// Since spans/events and events recorded near a crash are often necessary for diagnosing the failure,
/// `WorkerGuard` provides a mechanism to ensure that _all_ buffered logs are flushed to their output.
/// `WorkerGuard` should be assigned in the `main` function or whatever the entrypoint of the program is.
/// This will ensure that the guard will be dropped during an unwinding or when `main` exits
/// successfully.
///
/// # Examples
///
/// ``` rust
/// # #[clippy::allow(needless_doctest_main)]
/// fn main() {
/// # fn doc() {
///     let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
///     let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);
///     tracing::subscriber::with_default(subscriber.finish(), || {
///         // Emit some tracing events within context of the non_blocking `_guard` and tracing subscriber
///         tracing::event!(tracing::Level::INFO, "Hello");
///     });
/// // Exiting the context of `main` will drop the `_guard` and any remaining logs should get flushed
/// # }
/// }
/// ```
#[must_use]
#[derive(Debug)]
pub struct WorkerGuard {
    guard: Option<JoinHandle<()>>,
    shutdown_signal: Arc<AtomicBool>,
}

/// A non-blocking writer.
///
/// While the line between "blocking" and "non-blocking" IO is fuzzy, writing to a file is typically
/// considered to be a _blocking_ operation. For an application whose `Subscriber` writes spans and events
/// as they are emitted, an application might find the latency profile to be unacceptable.
/// `NonBlocking` moves the writing out of an application's data path by sending spans and events
/// to a dedicated logging thread.
///
/// This struct implements [`MakeWriter`][make_writer] from the `tracing-subscriber`
/// crate. Therefore, it can be used with the [`tracing_subscriber::fmt`][fmt] module
/// or with any other subscriber/layer implementation that uses the `MakeWriter` trait.
///
/// [make_writer]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/trait.MakeWriter.html
/// [fmt]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/index.html
#[derive(Clone, Debug)]
pub struct NonBlocking {
    error_counter: Arc<AtomicU64>,
    channel: Sender<Vec<u8>>,
    is_lossy: bool,
}

impl NonBlocking {
    /// Returns a new `NonBlocking` writer wrapping the provided `writer`.
    ///
    /// The returned `NonBlocking` writer will have the [default configuration][default] values.
    /// Other configurations can be specified using the [builder] interface.
    ///
    /// [default]: ./struct.NonBlockingBuilder.html#method.default
    /// [builder]: ./struct.NonBlockingBuilder.html
    pub fn new<T: Write + Send + Sync + 'static>(writer: T) -> (NonBlocking, WorkerGuard) {
        NonBlockingBuilder::default().finish(writer)
    }

    fn create<T: Write + Send + Sync + 'static>(
        writer: T,
        buffered_lines_limit: usize,
        is_lossy: bool,
    ) -> (NonBlocking, WorkerGuard) {
        let (sender, receiver) = bounded(buffered_lines_limit);
        let shutdown_signal = Arc::new(AtomicBool::new(false));

        let worker = Worker::new(receiver, writer, shutdown_signal.clone());
        let worker_guard = WorkerGuard::new(worker.worker_thread(), shutdown_signal);

        (
            Self {
                channel: sender,
                error_counter: Arc::new(AtomicU64::new(0)),
                is_lossy,
            },
            worker_guard,
        )
    }

    /// Returns a counter for the number of times logs where dropped. This will always return zero if
    /// `NonBlocking` is not lossy.
    pub fn error_counter(&self) -> Arc<AtomicU64> {
        self.error_counter.clone()
    }
}

/// A builder for [`NonBlocking`][non-blocking].
///
/// [non-blocking]: ./struct.NonBlocking.html
#[derive(Debug)]
pub struct NonBlockingBuilder {
    buffered_lines_limit: usize,
    is_lossy: bool,
}

impl NonBlockingBuilder {
    /// Sets the number of lines to buffer before dropping logs or exerting backpressure on senders
    pub fn buffered_lines_limit(mut self, buffered_lines_limit: usize) -> NonBlockingBuilder {
        self.buffered_lines_limit = buffered_lines_limit;
        self
    }

    /// Sets whether `NonBlocking` should be lossy or not.
    ///
    /// If set to `true`, logs will be dropped when the buffered limit is reached. If `false`, backpressure
    /// will be exerted on senders, blocking them until the buffer has capacity again.
    ///
    /// By default, the built `NonBlocking` will be lossy.
    pub fn lossy(mut self, is_lossy: bool) -> NonBlockingBuilder {
        self.is_lossy = is_lossy;
        self
    }

    /// Completes the builder, returning the configured `NonBlocking`.
    pub fn finish<T: Write + Send + Sync + 'static>(self, writer: T) -> (NonBlocking, WorkerGuard) {
        NonBlocking::create(writer, self.buffered_lines_limit, self.is_lossy)
    }
}

impl Default for NonBlockingBuilder {
    fn default() -> Self {
        NonBlockingBuilder {
            buffered_lines_limit: DEFAULT_BUFFERED_LINES_LIMIT,
            is_lossy: true,
        }
    }
}

impl std::io::Write for NonBlocking {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let buf_size = buf.len();
        if self.is_lossy {
            if self.channel.try_send(buf.to_vec()).is_err() {
                self.error_counter.fetch_add(1, Ordering::Relaxed);
            }
        } else {
            return match self.channel.send(buf.to_vec()) {
                Ok(_) => Ok(buf_size),
                Err(_) => Err(io::Error::from(io::ErrorKind::Other)),
            };
        }
        Ok(buf_size)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.write(buf).map(|_| ())
    }
}

impl MakeWriter for NonBlocking {
    type Writer = NonBlocking;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}

impl WorkerGuard {
    fn new(handle: JoinHandle<()>, shutdown_signal: Arc<AtomicBool>) -> Self {
        WorkerGuard {
            guard: Some(handle),
            shutdown_signal,
        }
    }

    fn stop(&mut self) -> std::thread::Result<()> {
        match self.guard.take() {
            Some(handle) => handle.join(),
            None => Ok(()),
        }
    }
}

impl Drop for WorkerGuard {
    fn drop(&mut self) {
        self.shutdown_signal.store(true, Ordering::Relaxed);
        match self.stop() {
            Ok(_) => (),
            Err(e) => println!("Failed to join worker thread. Error: {:?}", e),
        }
    }
}
