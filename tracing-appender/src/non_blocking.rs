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
/// If `NonBlocking` is configured to be lossy, it will drop any additional logs emitted when at
/// capacity. If it is not lossy, backpressure will be exerted on senders, causing them to wait 
/// until there is buffer capacity remaining before enqueuing new lines.
pub const DEFAULT_BUFFERED_LINES_LIMIT: usize = 128_000;

/// A guard which triggers an associated [`NonBlocking`] writer to flush logs when dropped.
///
/// Writing to a [`NonBlocking`] writer will **not** immediately write a log line to the underlying
/// output. Instead, the log line will be enqueued to be written by the logging worker thread. In
/// addition, to improve throughput, the non-blocking writer flushes the underlying output
/// periodically, rather than every time a line is written. This means that if the program 
/// terminates abruptly (such as by panicking, or by calling `std::process::exit`), some log lines 
/// may not be written.
///
/// Since logs recorded near a crash are often necessary for diagnosing the failure, this type 
/// provides a mechanism to ensure that all buffered logs are written to the output, and that the
/// output is flushed prior to terminating. In order for this to work, this guard should generally 
/// be held in the `main` function (or whatever the outermost scope of the program is). This will 
/// ensure that it is dropped when unwinding, or when `main` returns.
#[derive(Debug)]
pub struct WorkerGuard {
    guard: Option<JoinHandle<()>>,
    shutdown_signal: Arc<AtomicBool>,
}

/// A non-blocking writer.
///
/// Writing to an output, such as `std::io::stdout` or a file, is typically a blocking operation.
/// This means that a `Subscriber` where events are logged as they occur will block any threads 
/// emitting events until the events have been logged. This type provides a way to move logging 
/// out of an application's data path by sending log messages to a dedicated worker thread. Any
/// logs written to a `NonBlocking` writer will be forwarded to a corresponding worker to be output.
///
/// This struct implements the [`MakeWriter` trait][make_writer] from the `tracing-subscriber`
/// crate, and can be used with the [`tracing_subscriber::fmt`][fmt] module, or with any other 
/// subscriber implementation that uses the `MakeWriter` interface.
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

/// Builder for constructing [NonBlocking]: ./struct.NonBlocking.html
#[derive(Debug)]
pub struct NonBlockingBuilder {
    buffered_lines_limit: usize,
    is_lossy: bool,
}

impl NonBlockingBuilder {
    /// Sets the number of lines to buffer before dropping logs.
    pub fn buffered_lines_limit(mut self, buffered_lines_limit: usize) -> NonBlockingBuilder {
        self.buffered_lines_limit = buffered_lines_limit;
        self
    }

    /// Sets whether `NonBlocking` should be lossy or not. If set to `False`, `NonBlocking` will
    /// block on writing logs to the channel.
    pub fn lossy(mut self, is_lossy: bool) -> NonBlockingBuilder {
        self.is_lossy = is_lossy;
        self
    }

    /// Call to finish creation of `NonBlocking`
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
