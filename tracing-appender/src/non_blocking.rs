//! A non-blocking, off-thread writer.
//!
//! This spawns a dedicated worker thread which is responsible for writing log
//! lines to the provided writer. When a line is written using the returned
//! `NonBlocking` struct's `make_writer` method, it will be enqueued to be
//! written by the worker thread.
//!
//! The queue has a fixed capacity, and if it becomes full, any logs written
//! to it will be dropped until capacity is once again available. This may
//! occur if logs are consistently produced faster than the worker thread can
//! output them. The queue capacity and behavior when full (i.e., whether to
//! drop logs or to exert backpressure to slow down senders) can be configured
//! using [`NonBlockingBuilder::default()`][builder].
//! This function returns the default configuration. It is equivalent to:
//!
//! ```rust
//! # use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
//! # fn doc() -> (NonBlocking, WorkerGuard) {
//! tracing_appender::non_blocking(std::io::stdout())
//! # }
//! ```
//! [builder]: ./struct.NonBlockingBuilder.html#method.default
//!
//! <br/> This function returns a tuple of `NonBlocking` and `WorkerGuard`.
//! `NonBlocking` implements [`MakeWriter`] which integrates with `tracing_subscriber`.
//! `WorkerGuard` is a drop guard that is responsible for flushing any remaining logs when
//! the program terminates.
//!
//! Note that the `WorkerGuard` returned by `non_blocking` _must_ be assigned to a binding that
//! is not `_`, as `_` will result in the `WorkerGuard` being dropped immediately.
//! Unintentional drops of `WorkerGuard` remove the guarantee that logs will be flushed
//! during a program's termination, in a panic or otherwise.
//!
//! See [`WorkerGuard`][worker_guard] for examples of using the guard.
//!
//! [worker_guard]: ./struct.WorkerGuard.html
//!
//! # Examples
//!
//! ``` rust
//! # fn docs() {
//! let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
//! let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);
//! tracing::subscriber::with_default(subscriber.finish(), || {
//!    tracing::event!(tracing::Level::INFO, "Hello");
//! });
//! # }
//! ```
use crate::worker::Worker;
use crate::Msg;
use crossbeam_channel::{bounded, SendTimeoutError, Sender};
use std::io;
use std::io::Write;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
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
/// fn main () {
/// # fn doc() {
///     let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
///     let subscriber = tracing_subscriber::fmt().with_writer(non_blocking);
///     tracing::subscriber::with_default(subscriber.finish(), || {
///         // Emit some tracing events within context of the non_blocking `_guard` and tracing subscriber
///         tracing::event!(tracing::Level::INFO, "Hello");
///     });
///     // Exiting the context of `main` will drop the `_guard` and any remaining logs should get flushed
/// # }
/// }
/// ```
#[must_use]
#[derive(Debug)]
pub struct WorkerGuard {
    guard: Option<JoinHandle<()>>,
    sender: Sender<Msg>,
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
    channel: Sender<Msg>,
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

        let worker = Worker::new(receiver, writer);
        let worker_guard = WorkerGuard::new(worker.worker_thread(), sender.clone());

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
            if self.channel.try_send(Msg::Line(buf.to_vec())).is_err() {
                self.error_counter.fetch_add(1, Ordering::Relaxed);
            }
        } else {
            return match self.channel.send(Msg::Line(buf.to_vec())) {
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
    fn new(handle: JoinHandle<()>, sender: Sender<Msg>) -> Self {
        WorkerGuard {
            guard: Some(handle),
            sender,
        }
    }
}

impl Drop for WorkerGuard {
    fn drop(&mut self) {
        match self
            .sender
            .send_timeout(Msg::Shutdown, Duration::from_millis(100))
        {
            Ok(_) | Err(SendTimeoutError::Disconnected(_)) => (),
            Err(SendTimeoutError::Timeout(e)) => println!(
                "Failed to send shutdown signal to logging worker. Error: {:?}",
                e
            ),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::thread;
    use std::time::Duration;
    use std::sync::Mutex;
    use rand::Rng;

    struct MockWriter {
        writer: Arc<Mutex<Vec<String>>>,
        max_writes_allowed: usize,
        writes_attempted: usize,
    }

    impl Default for MockWriter {
        fn default() -> Self {
            MockWriter {
                writer: Arc::new(Mutex::new(Vec::new())),
                max_writes_allowed: 1,
                writes_attempted: 0,
            }
        }
    }

    impl std::io::Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let buf_len = buf.len();
            self.writes_attempted += 1;
            if self.writes_attempted > self.max_writes_allowed {
                return Err(std::io::Error::from(std::io::ErrorKind::WouldBlock));
            }

            self.writer.lock().expect("expected guard").push(String::from_utf8_lossy(buf).to_string());
            Ok(buf_len)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn logs_dropped_if_lossy() {
        let mock_writer = MockWriter::default();

        let (mut non_blocking, _guard) = self::NonBlockingBuilder::default()
            .lossy(true)
            .buffered_lines_limit(1)
            .finish(mock_writer);

        let error_count = non_blocking.error_counter();

        non_blocking
            .write("Hello".as_bytes())
            .expect("Failed to write");
        assert_eq!(0, error_count.load(Ordering::Relaxed));

        non_blocking
            .write(", World".as_bytes())
            .expect("Failed to write");
        assert_eq!(1, error_count.load(Ordering::Relaxed));

        non_blocking.write(".".as_bytes()).expect("Failed to write");
        assert_eq!(2, error_count.load(Ordering::Relaxed));
    }

    #[test]
    fn multi_threaded_writes() {
        let inner_writer = Arc::new(Mutex::new(Vec::new()));

        let mut mock_writer = MockWriter::default();
        mock_writer.max_writes_allowed = DEFAULT_BUFFERED_LINES_LIMIT;
        mock_writer.writer = inner_writer.clone();

        let (non_blocking, _guard) = self::NonBlockingBuilder::default()
            .lossy(true)
            .finish(mock_writer);

        let error_count = non_blocking.error_counter();
        let mut join_handles: Vec<JoinHandle<()>> = Vec::with_capacity(10);


        let subscriber = tracing_subscriber::fmt().with_writer(non_blocking.clone());

        tracing::subscriber::with_default(subscriber.finish(), || {
            for _ in 0..10 {
                let mut non_blocking_cloned = non_blocking.clone();
                join_handles.push(thread::spawn( move || {
                    // Sleep a random amount of time so that we can interleave the threads.
                    thread::sleep(Duration::from_millis(rand::thread_rng().gen_range(0, 1000)));
                    non_blocking_cloned.write(format!("Hello").as_bytes()).expect("Failed to write hello from thread");
                }));
            }
        });

        for handle in join_handles {
            handle.join().expect("Failed to join thread");
        }

        let mut hello_count: u8 = 0;
        match inner_writer.lock() {
            Ok(guard) => {
                for msg in guard.iter() {
                    if msg.as_str().eq("Hello") {
                        hello_count += 1;
                    }
                }
            },
            Err(_) => {assert!(false)},
        }

        drop(non_blocking);
        assert_eq!(10, hello_count);
        assert_eq!(0, error_count.load(Ordering::Relaxed));
    }
}
