//! Abstractions for creating [`io::Write`] instances.

use super::Builder;
use std::io;

/// A type that can create [`io::Write`] instances.
///
/// `NewWriter` is used by [`FmtSubscriber`] to print formatted text representations of [`Event`]s.
///
/// This trait is already implemented for function pointers and immutably-borrowing closures that
/// return an instance of [`io::Write`], such as [`io::stdout`] and [`io::stderr`].
///
/// [`FmtSubscriber`]: crate::FmtSubscriber
/// [`Event`]: tracing_core::Event
pub trait NewWriter {
    /// The concrete [`io::Write`] implementation returned by [`new_writer`].
    ///
    /// [`new_writer`]: NewWriter::new_writer
    type Writer: io::Write;

    /// Returns an instance of [`Writer`].
    ///
    /// # Implementer notes
    ///
    /// [`FmtSubscriber`] will call this method each time an event is recorded. Ensure any state
    /// that must be saved across writes is not lost when the [`Writer`] instance is dropped. If
    /// creating a [`io::Write`] instance is expensive, be sure to cache it when implementing
    /// [`NewWriter`] to improve performance.
    ///
    /// [`Writer`]: NewWriter::Writer
    /// [`FmtSubscriber`]: crate::FmtSubscriber
    fn new_writer(&self) -> Self::Writer;
}

impl<F, W> NewWriter for F
where
    F: Fn() -> W,
    W: io::Write,
{
    type Writer = W;

    fn new_writer(&self) -> Self::Writer {
        (self)()
    }
}

impl<N, E, F, W> Builder<N, E, F, W> {
    /// Sets the [`NewWriter`] that the subscriber being built will use to write events.
    pub fn with_writer<W2>(self, new_writer: W2) -> Builder<N, E, F, W2>
    where
        W2: NewWriter + 'static,
    {
        Builder {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event,
            filter: self.filter,
            settings: self.settings,
            new_writer,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{FmtSubscriber, NewWriter};
    use std::io;
    use std::sync::{Mutex, MutexGuard, TryLockError};
    use tracing::error;
    use tracing_core::dispatcher::{self, Dispatch};

    fn test_writer<T: NewWriter + Send + Sync + 'static>(
        new_writer: T,
        msg: &str,
        buf: &Mutex<Vec<u8>>,
    ) {
        let subscriber = FmtSubscriber::builder()
            .with_writer(new_writer)
            .without_time()
            .with_ansi(false)
            .finish();
        let dispatch = Dispatch::from(subscriber);

        dispatcher::with_default(&dispatch, || {
            error!("{}", msg);
        });

        // TODO: remove time ANSI codes when `crate::time::write` respects `with_ansi(false)`
        let expected = format!(
            "\u{1b}[2m\u{1b}[0mERROR tracing_fmt::writer::test: {}\n",
            msg
        );
        let actual = String::from_utf8(buf.try_lock().unwrap().to_vec()).unwrap();
        assert_eq!(actual, expected);
    }

    struct MockWriter<'a> {
        buf: &'a Mutex<Vec<u8>>,
    }

    impl<'a> MockWriter<'a> {
        fn new(buf: &'a Mutex<Vec<u8>>) -> Self {
            Self { buf }
        }

        fn map_error<Guard>(err: TryLockError<Guard>) -> io::Error {
            match err {
                TryLockError::WouldBlock => io::Error::from(io::ErrorKind::WouldBlock),
                TryLockError::Poisoned(_) => io::Error::from(io::ErrorKind::Other),
            }
        }

        fn buf(&self) -> io::Result<MutexGuard<'a, Vec<u8>>> {
            self.buf.try_lock().map_err(Self::map_error)
        }
    }

    impl<'a> io::Write for MockWriter<'a> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buf()?.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.buf()?.flush()
        }
    }

    struct MockNewWriter<'a> {
        buf: &'a Mutex<Vec<u8>>,
    }

    impl<'a> MockNewWriter<'a> {
        fn new(buf: &'a Mutex<Vec<u8>>) -> Self {
            Self { buf }
        }
    }

    impl<'a> NewWriter for MockNewWriter<'a> {
        type Writer = MockWriter<'a>;

        fn new_writer(&self) -> Self::Writer {
            MockWriter::new(self.buf)
        }
    }

    #[test]
    fn custom_writer_closure() {
        lazy_static! {
            static ref BUF: Mutex<Vec<u8>> = Mutex::new(vec![]);
        }

        let new_writer = || MockWriter::new(&BUF);
        let msg = "my custom writer closure error";
        test_writer(new_writer, msg, &BUF);
    }

    #[test]
    fn custom_writer_struct() {
        lazy_static! {
            static ref BUF: Mutex<Vec<u8>> = Mutex::new(vec![]);
        }

        let new_writer = MockNewWriter::new(&BUF);
        let msg = "my custom writer struct error";
        test_writer(new_writer, msg, &BUF);
    }
}
