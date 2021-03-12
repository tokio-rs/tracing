//! Abstractions for creating [`io::Write`] instances.
//!
//! [`io::Write`]: std::io::Write

use std::{
    fmt::Debug,
    io::{self, Write},
    sync::{Mutex, MutexGuard},
};
use tracing_core::Metadata;

/// A type that can create [`io::Write`] instances.
///
/// `MakeWriter` is used by [`fmt::Collector`] or [`fmt::Subscriber`] to print
/// formatted text representations of [`Event`]s.
///
/// This trait is already implemented for function pointers and
/// immutably-borrowing closures that return an instance of [`io::Write`], such
/// as [`io::stdout`] and [`io::stderr`]. Additionally, it is implemented for
/// [`std::sync::Mutex`][mutex] when the type inside the mutex implements
/// [`io::Write`].
///
/// The [`MakeWriter::make_writer_for`] method takes [`Metadata`] describing a
/// span or event and returns a writer. `MakeWriter`s can optionally provide
/// implementations of this method with behaviors that differ based on the span
/// or event being written. For example, events at different [levels] might be
/// written to different output streams, or data from different [targets] might
/// be written to separate log files. When the `MakeWriter` has no custom
/// behavior based on metadata, the default implementation of `make_writer_for`
/// simply calls `self.make_writer()`, ignoring the metadata. Therefore, when
/// metadata _is_ available, callers should prefer to call `make_writer_for`,
/// passing in that metadata, so that the `MakeWriter` implementation can choose
/// the appropriate behavior.
///
/// # Examples
///
/// The simplest usage is to pass in a named function that returns a writer. For
/// example, to log all events to stderr, we could write:
/// ```
/// let subscriber = tracing_subscriber::fmt()
///     .with_writer(std::io::stderr)
///     .finish();
/// # drop(subscriber);
/// ```
///
/// Any function that returns a writer can be used:
///
/// ```
/// fn make_my_great_writer() -> impl std::io::Write {
///     // ...
///     # std::io::stdout()
/// }
///
/// let subscriber = tracing_subscriber::fmt()
///     .with_writer(make_my_great_writer)
///     .finish();
/// # drop(subscriber);
/// ```
///
/// A closure can be used to introduce arbitrary logic into how the writer is
/// created. Consider the (admittedly rather silly) example of sending every 5th
/// event to stderr, and all other events to stdout:
///
/// ```
/// use std::io;
/// use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
///
/// let n = AtomicUsize::new(0);
/// let subscriber = tracing_subscriber::fmt()
///     .with_writer(move || -> Box<dyn io::Write> {
///         if n.fetch_add(1, Relaxed) % 5 == 0 {
///             Box::new(io::stderr())
///         } else {
///             Box::new(io::stdout())
///        }
///     })
///     .finish();
/// # drop(subscriber);
/// ```
///
/// A single instance of a type implementing [`io::Write`] may be used as a
/// `MakeWriter` by wrapping it in a [`Mutex`][mutex]. For example, we could
/// write to a file like so:
///
/// ```
/// use std::{fs::File, sync::Mutex};
///
/// # fn docs() -> Result<(), Box<dyn std::error::Error>> {
/// let log_file = File::create("my_cool_trace.log")?;
/// let subscriber = tracing_subscriber::fmt()
///     .with_writer(Mutex::new(log_file))
///     .finish();
/// # drop(subscriber);
/// # Ok(())
/// # }
/// ```
///
/// [`io::Write`]: std::io::Write
/// [`fmt::Collector`]: super::super::fmt::Collector
/// [`fmt::Subscriber`]: super::super::fmt::Subscriber
/// [`Event`]: tracing_core::event::Event
/// [`io::stdout`]: std::io::stdout()
/// [`io::stderr`]: std::io::stderr()
/// [mutex]: std::sync::Mutex
/// [`MakeWriter::make_writer_for`]: MakeWriter::make_writer_for
/// [`Metadata`]: tracing_core::Metadata
/// [levels]: tracing_core::Level
/// [targets]: tracing_core::Metadata::target
pub trait MakeWriter<'a> {
    /// The concrete [`io::Write`] implementation returned by [`make_writer`].
    ///
    /// [`io::Write`]: std::io::Write
    /// [`make_writer`]: MakeWriter::make_writer
    type Writer: io::Write;

    /// Returns an instance of [`Writer`].
    ///
    /// # Implementer notes
    ///
    /// [`fmt::Subscriber`] or [`fmt::Collector`] will call this method each
    /// time an event is recorded. Ensure any state that must be saved across
    /// writes is not lost when the [`Writer`] instance is dropped. If creating
    /// a [`io::Write`] instance is expensive, be sure to cache it when
    /// implementing [`MakeWriter`] to improve performance.
    ///
    /// [`Writer`]: MakeWriter::Writer
    /// [`fmt::Subscriber`]: super::super::fmt::Subscriber
    /// [`fmt::Collector`]: super::super::fmt::Collector
    /// [`io::Write`]: std::io::Write
    fn make_writer(&'a self) -> Self::Writer;

    /// Returns a [`Writer`] for writing data from the span or event described
    /// by the provided [`Metadata`].
    ///
    /// By default, this calls [`self.make_writer()`][make_writer], ignoring
    /// the provided metadata, but implementations can override this to provide
    /// metadata-specific behaviors.
    ///
    /// This method allows `MakeWriter` implementations to implement different
    /// behaviors based on the span or event being written. The `MakeWriter`
    /// type might return different writers based on the provided metadata, or
    /// might write some values to the writer before or after providing it to
    /// the caller.
    ///
    /// For example, we might want to write data from spans and events at the
    /// [`ERROR`] and [`WARN`] levels to `stderr`, and data from spans or events
    /// at lower levels to stdout:
    ///
    /// ```
    /// use std::io::{self, Stdout, Stderr, StdoutLock, StderrLock};
    /// use tracing_subscriber::fmt::writer::MakeWriter;
    /// use tracing_core::{Metadata, Level};
    ///
    /// pub struct MyMakeWriter {
    ///     stdout: Stdout,
    ///     stderr: Stderr,
    /// }
    ///
    /// /// A lock on either stdout or stderr, depending on the verbosity level
    /// /// of the event being written.
    /// pub enum StdioLock<'a> {
    ///     Stdout(StdoutLock<'a>),
    ///     Stderr(StderrLock<'a>),
    /// }
    ///
    /// impl<'a> io::Write for StdioLock<'a> {
    ///     fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    ///         match self {
    ///             StdioLock::Stdout(lock) => lock.write(buf),
    ///             StdioLock::Stderr(lock) => lock.write(buf),
    ///         }
    ///     }
    ///
    ///     fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
    ///         // ...
    ///         # match self {
    ///         #     StdioLock::Stdout(lock) => lock.write_all(buf),
    ///         #     StdioLock::Stderr(lock) => lock.write_all(buf),
    ///         # }
    ///     }
    ///
    ///     fn flush(&mut self) -> io::Result<()> {
    ///         // ...
    ///         # match self {
    ///         #     StdioLock::Stdout(lock) => lock.flush(),
    ///         #     StdioLock::Stderr(lock) => lock.flush(),
    ///         # }
    ///     }
    /// }
    ///
    /// impl<'a> MakeWriter<'a> for MyMakeWriter {
    ///     type Writer = StdioLock<'a>;
    ///
    ///     fn make_writer(&'a self) -> Self::Writer {
    ///         // We must have an implementation of `make_writer` that makes
    ///         // a "default" writer without any configuring metadata. Let's
    ///         // just return stdout in that case.
    ///         StdioLock::Stdout(self.stdout.lock())
    ///     }
    ///
    ///     fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
    ///         // Here's where we can implement our special behavior. We'll
    ///         // check if the metadata's verbosity level is WARN or ERROR,
    ///         // and return stderr in that case.
    ///         if meta.level() <= &Level::WARN {
    ///             return StdioLock::Stderr(self.stderr.lock());
    ///         }
    ///
    ///         // Otherwise, we'll return stdout.
    ///         StdioLock::Stdout(self.stdout.lock())
    ///     }
    /// }
    /// ```
    ///
    /// [`Writer`]: MakeWriter::Writer
    /// [`Metadata`]: tracing_core::Metadata
    /// [make_writer]: MakeWriter::make_writer
    /// [`WARN`]: tracing_core::Level::WARN
    /// [`ERROR`]: tracing_core::Level::ERROR
    fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
        let _ = meta;
        self.make_writer()
    }
}

/// A type implementing [`io::Write`] for a [`MutexGuard`] where the type
/// inside the [`Mutex`] implements [`io::Write`].
///
/// This is used by the [`MakeWriter`] implementation for [`Mutex`], because
/// [`MutexGuard`] itself will not implement [`io::Write`] — instead, it
/// _dereferences_ to a type implementing [`io::Write`]. Because [`MakeWriter`]
/// requires the `Writer` type to implement [`io::Write`], it's necessary to add
/// a newtype that forwards the trait implementation.
///
/// [`io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
/// [`MutexGuard`]: https://doc.rust-lang.org/std/sync/struct.MutexGuard.html
/// [`Mutex`]: https://doc.rust-lang.org/std/sync/struct.Mutex.html
/// [`MakeWriter`]: trait.MakeWriter.html
#[derive(Debug)]
pub struct MutexGuardWriter<'a, W>(MutexGuard<'a, W>);

/// A writer intended to support [`libtest`'s output capturing][capturing] for use in unit tests.
///
/// `TestWriter` is used by [`fmt::Collector`] or [`fmt::Subscriber`] to enable capturing support.
///
/// `cargo test` can only capture output from the standard library's [`print!`] macro. See
/// [`libtest`'s output capturing][capturing] for more details about output capturing.
///
/// Writing to [`io::stdout`] and [`io::stderr`] produces the same results as using
/// [`libtest`'s `--nocapture` option][nocapture] which may make the results look unreadable.
///
/// [`fmt::Collector`]: super::Collector
/// [`fmt::Subscriber`]: super::Subscriber
/// [capturing]: https://doc.rust-lang.org/book/ch11-02-running-tests.html#showing-function-output
/// [nocapture]: https://doc.rust-lang.org/cargo/commands/cargo-test.html
/// [`io::stdout`]: std::io::stdout()
/// [`io::stderr`]: std::io::stderr()
/// [`print!`]: std::print!
#[derive(Default, Debug)]
pub struct TestWriter {
    _p: (),
}

/// A writer that erases the specific [`io::Write`] and [`MakeWriter`] types being used.
///
/// This is useful in cases where the concrete type of the writer cannot be known
/// until runtime.
///
/// # Examples
///
/// A function that returns a [`Collect`] that will write to either stdout or stderr:
///
/// ```rust
/// # use tracing::Collect;
/// # use tracing_subscriber::fmt::writer::BoxMakeWriter;
///
/// fn dynamic_writer(use_stderr: bool) -> impl Collect {
///     let writer = if use_stderr {
///         BoxMakeWriter::new(std::io::stderr)
///     } else {
///         BoxMakeWriter::new(std::io::stdout)
///     };
///
///     tracing_subscriber::fmt().with_writer(writer).finish()
/// }
/// ```
///
/// [`Collect`]: tracing::Collect
/// [`io::Write`]: std::io::Write
pub struct BoxMakeWriter {
    inner: Box<dyn for<'a> MakeWriter<'a, Writer = Box<dyn Write + 'a>> + Send + Sync>,
}

impl<'a, F, W> MakeWriter<'a> for F
where
    F: Fn() -> W,
    W: io::Write,
{
    type Writer = W;

    fn make_writer(&'a self) -> Self::Writer {
        (self)()
    }
}

// === impl TestWriter ===

impl TestWriter {
    /// Returns a new `TestWriter` with the default configuration.
    pub fn new() -> Self {
        Self::default()
    }
}

impl io::Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let out_str = String::from_utf8_lossy(buf);
        print!("{}", out_str);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for TestWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        Self::default()
    }
}

// === impl BoxMakeWriter ===

impl BoxMakeWriter {
    /// Constructs a `BoxMakeWriter` wrapping a type implementing [`MakeWriter`].
    ///
    pub fn new<M>(make_writer: M) -> Self
    where
        M: for<'a> MakeWriter<'a> + Send + Sync + 'static,
    {
        Self {
            inner: Box::new(Boxed(make_writer)),
        }
    }
}

impl Debug for BoxMakeWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad("BoxMakeWriter { ... }")
    }
}

impl<'a> MakeWriter<'a> for BoxMakeWriter {
    type Writer = Box<dyn Write + 'a>;

    fn make_writer(&'a self) -> Self::Writer {
        self.inner.make_writer()
    }

    fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
        self.inner.make_writer_for(meta)
    }
}

struct Boxed<M>(M);

impl<'a, M> MakeWriter<'a> for Boxed<M>
where
    M: MakeWriter<'a>,
{
    type Writer = Box<dyn Write + 'a>;

    fn make_writer(&'a self) -> Self::Writer {
        let w = self.0.make_writer();
        Box::new(w)
    }

    fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
        let w = self.0.make_writer_for(meta);
        Box::new(w)
    }
}

// === impl Mutex/MutexGuardWriter ===

impl<'a, W> MakeWriter<'a> for Mutex<W>
where
    W: io::Write + 'a,
{
    type Writer = MutexGuardWriter<'a, W>;

    fn make_writer(&'a self) -> Self::Writer {
        MutexGuardWriter(self.lock().expect("lock poisoned"))
    }
}

impl<'a, W> io::Write for MutexGuardWriter<'a, W>
where
    W: io::Write,
{
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        self.0.write_vectored(bufs)
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.0.write_all(buf)
    }

    #[inline]
    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> io::Result<()> {
        self.0.write_fmt(fmt)
    }
}

#[cfg(test)]
mod test {
    use super::MakeWriter;
    use crate::fmt::format::Format;
    use crate::fmt::test::{MockMakeWriter, MockWriter};
    use crate::fmt::Collector;
    use std::sync::{Arc, Mutex};
    use tracing::error;
    use tracing_core::dispatch::{self, Dispatch};

    fn test_writer<T>(make_writer: T, msg: &str, buf: &Mutex<Vec<u8>>)
    where
        T: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
    {
        let subscriber = {
            #[cfg(feature = "ansi")]
            {
                let f = Format::default().without_time().with_ansi(false);
                Collector::builder()
                    .event_format(f)
                    .with_writer(make_writer)
                    .finish()
            }
            #[cfg(not(feature = "ansi"))]
            {
                let f = Format::default().without_time();
                Collector::builder()
                    .event_format(f)
                    .with_writer(make_writer)
                    .finish()
            }
        };
        let dispatch = Dispatch::from(subscriber);

        dispatch::with_default(&dispatch, || {
            error!("{}", msg);
        });

        let expected = format!("ERROR {}: {}\n", module_path!(), msg);
        let actual = String::from_utf8(buf.try_lock().unwrap().to_vec()).unwrap();
        assert!(actual.contains(expected.as_str()));
    }

    #[test]
    fn custom_writer_closure() {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let buf2 = buf.clone();
        let make_writer = move || MockWriter::new(buf2.clone());
        let msg = "my custom writer closure error";
        test_writer(make_writer, msg, &buf);
    }

    #[test]
    fn custom_writer_struct() {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let make_writer = MockMakeWriter::new(buf.clone());
        let msg = "my custom writer struct error";
        test_writer(make_writer, msg, &buf);
    }

    #[test]
    fn custom_writer_mutex() {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer = MockWriter::new(buf.clone());
        let make_writer = Mutex::new(writer);
        let msg = "my mutex writer error";
        test_writer(make_writer, msg, &buf);
    }
}
