use super::Builder;
use std::io;

pub trait NewWriter {
    type Writer: io::Write;

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
    /// Sets the Writer that the subscriber being built will use to log events.
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
    use crate::*;
    use std::io;
    use std::sync::{Mutex, MutexGuard, TryLockError};
    use tracing::error;
    use tracing_core::dispatcher::{self, Dispatch};

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
    fn customize_writer() {
        lazy_static! {
            static ref BUF: Mutex<Vec<u8>> = Mutex::new(vec![]);
        }

        let new_writer = || MockWriter::new(&BUF);
        let subscriber = FmtSubscriber::builder()
            .with_writer(new_writer)
            .without_time()
            .with_ansi(false)
            .finish();
        let dispatch = Dispatch::from(subscriber);

        fn event() {
            error!("my customized writer error");
        }

        dispatcher::with_default(&dispatch, || {
            event();
        });

        // TODO: remove time ANSI codes when `time::write` respects `with_ansi(false)`
        let expected =
            "\u{1b}[2m\u{1b}[0mERROR tracing_fmt::writer::test: my customized writer error\n";
        let actual = String::from_utf8(BUF.try_lock().unwrap().to_vec()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn custom_writer_struct() {
        lazy_static! {
            static ref BUF: Mutex<Vec<u8>> = Mutex::new(vec![]);
        }

        let new_writer = MockNewWriter::new(&BUF);
        let subscriber = FmtSubscriber::builder()
            .with_writer(new_writer)
            .without_time()
            .with_ansi(false)
            .finish();
        let dispatch = Dispatch::from(subscriber);

        fn event() {
            error!("my customized writer struct error");
        }

        dispatcher::with_default(&dispatch, || {
            event();
        });

        // TODO: remove time ANSI codes when `time::write` respects `with_ansi(false)`
        let expected =
            "\u{1b}[2m\u{1b}[0mERROR tracing_fmt::writer::test: my customized writer struct error\n";
        let actual = String::from_utf8(BUF.try_lock().unwrap().to_vec()).unwrap();
        assert_eq!(actual, expected);
    }
}
