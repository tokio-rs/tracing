//! Abstractions for creating [`io::Write`] instances.
//!
//! [`io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
use std::{
    fmt::Debug,
    io::{self, Write},
};
use tracing_core::Metadata;

/// A type that can create [`io::Write`] instances.
///
/// `MakeWriter` is used by [`fmt::Subscriber`] or [`fmt::Layer`] to print formatted text representations of
/// [`Event`]s.
///
/// This trait is already implemented for function pointers and immutably-borrowing closures that
/// return an instance of [`io::Write`], such as [`io::stdout`] and [`io::stderr`].
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
pub trait MakeWriter {
    /// The concrete [`io::Write`] implementation returned by [`make_writer`].
    ///
    /// [`io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
    /// [`make_writer`]: #tymethod.make_writer
    type Writer: io::Write;

    /// Returns an instance of [`Writer`].
    ///
    /// # Implementer notes
    ///
    /// [`fmt::Layer`] or [`fmt::Subscriber`] will call this method each time an event is recorded. Ensure any state
    /// that must be saved across writes is not lost when the [`Writer`] instance is dropped. If
    /// creating a [`io::Write`] instance is expensive, be sure to cache it when implementing
    /// [`MakeWriter`] to improve performance.
    ///
    /// [`Writer`]: #associatedtype.Writer
    /// [`fmt::Layer`]: crate::fmt::Layer
    /// [`fmt::Subscriber`]: crate::fmt::Subscriber
    /// [`io::Write`]: std::io::Write
    fn make_writer(&self) -> Self::Writer;

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
    /// use std::io::{self, Stdout, Stderr};
    /// use tracing_subscriber::fmt::writer::MakeWriter;
    /// use tracing_core::{Metadata, Level};
    ///
    /// pub struct MyMakeWriter {}
    ///
    /// /// A lock on either stdout or stderr, depending on the verbosity level
    /// /// of the event being written.
    /// pub enum Stdio {
    ///     Stdout(Stdout),
    ///     Stderr(Stderr),
    /// }
    ///
    /// impl io::Write for Stdio {
    ///     fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    ///         match self {
    ///             Stdio::Stdout(io) => io.write(buf),
    ///             Stdio::Stderr(io) => io.write(buf),
    ///         }
    ///     }
    ///
    ///     fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
    ///         // ...
    ///         # match self {
    ///         #     Stdio::Stdout(io) => io.write_all(buf),
    ///         #     Stdio::Stderr(io) => io.write_all(buf),
    ///         # }
    ///     }
    ///
    ///     fn flush(&mut self) -> io::Result<()> {
    ///         // ...
    ///         # match self {
    ///         #     Stdio::Stdout(io) => io.flush(),
    ///         #     Stdio::Stderr(io) => io.flush(),
    ///         # }
    ///     }
    /// }
    ///
    /// impl MakeWriter for MyMakeWriter {
    ///     type Writer = Stdio;
    ///
    ///     fn make_writer(&self) -> Self::Writer {
    ///         // We must have an implementation of `make_writer` that makes
    ///         // a "default" writer without any configuring metadata. Let's
    ///         // just return stdout in that case.
    ///         Stdio::Stdout(io::stdout())
    ///     }
    ///
    ///     fn make_writer_for(&self, meta: &Metadata<'_>) -> Self::Writer {
    ///         // Here's where we can implement our special behavior. We'll
    ///         // check if the metadata's verbosity level is WARN or ERROR,
    ///         // and return stderr in that case.
    ///         if meta.level() <= &Level::WARN {
    ///             return Stdio::Stderr(io::stderr());
    ///         }
    ///
    ///         // Otherwise, we'll return stdout.
    ///         Stdio::Stdout(io::stdout())
    ///     }
    /// }
    /// ```
    ///
    /// [`Writer`]: MakeWriter::Writer
    /// [`Metadata`]: tracing_core::Metadata
    /// [make_writer]: MakeWriter::make_writer
    /// [`WARN`]: tracing_core::Level::WARN
    /// [`ERROR`]: tracing_core::Level::ERROR
    fn make_writer_for(&self, meta: &Metadata<'_>) -> Self::Writer {
        let _ = meta;
        self.make_writer()
    }
}

impl<F, W> MakeWriter for F
where
    F: Fn() -> W,
    W: io::Write,
{
    type Writer = W;

    fn make_writer(&self) -> Self::Writer {
        (self)()
    }
}

/// A writer intended to support [`libtest`'s output capturing][capturing] for use in unit tests.
///
/// `TestWriter` is used by [`fmt::Subscriber`] or [`fmt::Layer`] to enable capturing support.
///
/// `cargo test` can only capture output from the standard library's [`print!`] macro. See
/// [`libtest`'s output capturing][capturing] for more details about output capturing.
///
/// Writing to [`io::stdout`] and [`io::stderr`] produces the same results as using
/// [`libtest`'s `--nocapture` option][nocapture] which may make the results look unreadable.
///
/// [`fmt::Subscriber`]: ../struct.Subscriber.html
/// [`fmt::Layer`]: ../struct.Layer.html
/// [capturing]: https://doc.rust-lang.org/book/ch11-02-running-tests.html#showing-function-output
/// [nocapture]: https://doc.rust-lang.org/cargo/commands/cargo-test.html
/// [`io::stdout`]: https://doc.rust-lang.org/std/io/fn.stdout.html
/// [`io::stderr`]: https://doc.rust-lang.org/std/io/fn.stderr.html
/// [`print!`]: https://doc.rust-lang.org/std/macro.print.html
#[derive(Default, Debug)]
pub struct TestWriter {
    _p: (),
}

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

impl MakeWriter for TestWriter {
    type Writer = Self;

    fn make_writer(&self) -> Self::Writer {
        Self::default()
    }
}

/// A writer that erases the specific [`io::Write`] and [`Makewriter`] types being used.
///
/// This is useful in cases where the concrete type of the writer cannot be known
/// until runtime.
///
/// # Examples
///
/// A function that returns a [`Subscriber`] that will write to either stdout or stderr:
///
/// ```rust
/// # use tracing::Subscriber;
/// # use tracing_subscriber::fmt::writer::BoxMakeWriter;
///
/// fn dynamic_writer(use_stderr: bool) -> impl Subscriber {
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
/// [`Subscriber`]: https://docs.rs/tracing/latest/tracing/trait.Subscriber.html
/// [`io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
pub struct BoxMakeWriter {
    inner: Box<dyn MakeWriter<Writer = Box<dyn Write>> + Send + Sync>,
}

impl BoxMakeWriter {
    /// Constructs a `BoxMakeWriter` wrapping a type implementing [`MakeWriter`].
    ///
    /// [`MakeWriter`]: trait.MakeWriter.html
    pub fn new<M>(make_writer: M) -> Self
    where
        M: MakeWriter + Send + Sync + 'static,
        M::Writer: Write + 'static,
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

impl MakeWriter for BoxMakeWriter {
    type Writer = Box<dyn Write>;

    fn make_writer(&self) -> Self::Writer {
        self.inner.make_writer()
    }

    fn make_writer_for(&self, meta: &Metadata<'_>) -> Self::Writer {
        self.inner.make_writer_for(meta)
    }
}

struct Boxed<M>(M);

impl<M> MakeWriter for Boxed<M>
where
    M: MakeWriter,
    M::Writer: Write + 'static,
{
    type Writer = Box<dyn Write>;

    fn make_writer(&self) -> Self::Writer {
        let w = self.0.make_writer();
        Box::new(w)
    }

    fn make_writer_for(&self, meta: &Metadata<'_>) -> Self::Writer {
        let w = self.0.make_writer_for(meta);
        Box::new(w)
    }
}

#[cfg(test)]
mod test {
    use super::MakeWriter;
    use crate::fmt::format::Format;
    use crate::fmt::test::{MockMakeWriter, MockWriter};
    use crate::fmt::Subscriber;
    use lazy_static::lazy_static;
    use std::sync::Mutex;
    use tracing::error;
    use tracing_core::dispatcher::{self, Dispatch};

    fn test_writer<T>(make_writer: T, msg: &str, buf: &Mutex<Vec<u8>>)
    where
        T: MakeWriter + Send + Sync + 'static,
    {
        let subscriber = {
            #[cfg(feature = "ansi")]
            {
                let f = Format::default().without_time().with_ansi(false);
                Subscriber::builder()
                    .event_format(f)
                    .with_writer(make_writer)
                    .finish()
            }
            #[cfg(not(feature = "ansi"))]
            {
                let f = Format::default().without_time();
                Subscriber::builder()
                    .event_format(f)
                    .with_writer(make_writer)
                    .finish()
            }
        };
        let dispatch = Dispatch::from(subscriber);

        dispatcher::with_default(&dispatch, || {
            error!("{}", msg);
        });

        let expected = format!("ERROR {}: {}\n", module_path!(), msg);
        let actual = String::from_utf8(buf.try_lock().unwrap().to_vec()).unwrap();
        assert!(actual.contains(expected.as_str()));
    }

    #[test]
    fn custom_writer_closure() {
        lazy_static! {
            static ref BUF: Mutex<Vec<u8>> = Mutex::new(vec![]);
        }

        let make_writer = || MockWriter::new(&BUF);
        let msg = "my custom writer closure error";
        test_writer(make_writer, msg, &BUF);
    }

    #[test]
    fn custom_writer_struct() {
        lazy_static! {
            static ref BUF: Mutex<Vec<u8>> = Mutex::new(vec![]);
        }

        let make_writer = MockMakeWriter::new(&BUF);
        let msg = "my custom writer struct error";
        test_writer(make_writer, msg, &BUF);
    }
}
