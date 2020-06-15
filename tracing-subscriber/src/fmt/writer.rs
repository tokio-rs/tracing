//! Abstractions for creating [`io::Write`] instances.
//!
//! [`io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
use std::{
    fmt::Debug,
    io::{self, Write},
    sync::{Mutex, MutexGuard},
};

/// A type that can create [`io::Write`] instances.
///
/// `MakeWriter` is used by [`fmt::Collector`] or [`fmt::Subscriber`] to print
/// formatted text representations of [`Event`]s.
///
/// This trait is already implemented for function pointers and
/// immutably-borrowing closures that return an instance of [`io::Write`], such
/// as [`io::stdout`] and [`io::stderr`]. Additionally, it is implemented for
/// [`std::sync::Mutex`][mutex] when the tyoe inside the mutex implements
/// [`io::Write`].
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
/// created. For example, this will send every 5th event to stderr, and all
/// other events to stdout (why you would actually want to do this, I have no
/// idea, but you _can_):
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
/// [`io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
/// [`fmt::Collector`]: ../../fmt/struct.Collector.html
/// [`fmt::Subscriber`]: ../../fmt/struct.Subscriber.html
/// [`Event`]: https://docs.rs/tracing-core/0.1.5/tracing_core/event/struct.Event.html
/// [`io::stdout`]: https://doc.rust-lang.org/std/io/fn.stdout.html
/// [`io::stderr`]: https://doc.rust-lang.org/std/io/fn.stderr.html
/// [mutex]: https://doc.rust-lang.org/std/sync/struct.Mutex.html
pub trait MakeWriter<'a> {
    /// The concrete [`io::Write`] implementation returned by [`make_writer`].
    ///
    /// [`io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
    /// [`make_writer`]: #tymethod.make_writer
    type Writer: io::Write;

    /// Returns an instance of [`Writer`].
    ///
    /// # Implementer notes
    ///
    /// [`fmt::Subscriber`] or [`fmt::Collector`] will call this method each time an event is recorded. Ensure any state
    /// that must be saved across writes is not lost when the [`Writer`] instance is dropped. If
    /// creating a [`io::Write`] instance is expensive, be sure to cache it when implementing
    /// [`MakeWriter`] to improve performance.
    ///
    /// [`Writer`]: #associatedtype.Writer
    /// [`fmt::Subscriber`]: ../../fmt/struct.Subscriber.html
    /// [`fmt::Collector`]: ../../fmt/struct.Collector.html
    /// [`io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
    /// [`MakeWriter`]: trait.MakeWriter.html
    fn make_writer(&'a self) -> Self::Writer;
}

/// A type implementing [`io::Write`] for a [`MutexGuard`] where tyhe type
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
/// [`fmt::Collector`]: ../struct.Collector.html
/// [`fmt::Subscriber`]: ../struct.Subscriber.html
/// [capturing]: https://doc.rust-lang.org/book/ch11-02-running-tests.html#showing-function-output
/// [nocapture]: https://doc.rust-lang.org/cargo/commands/cargo-test.html
/// [`io::stdout`]: https://doc.rust-lang.org/std/io/fn.stdout.html
/// [`io::stderr`]: https://doc.rust-lang.org/std/io/fn.stderr.html
/// [`print!`]: https://doc.rust-lang.org/std/macro.print.html
#[derive(Default, Debug)]
pub struct TestWriter {
    _p: (),
}

/// A writer that erases the specific [`io::Write`] and [`Makewriter`] types being used.
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
/// [`MakeWriter`]: trait.MakeWriter.html
/// [`Collect`]: https://docs.rs/tracing/latest/tracing/trait.Collect.html
/// [`io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
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
    /// [`MakeWriter`]: trait.MakeWriter.html
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
