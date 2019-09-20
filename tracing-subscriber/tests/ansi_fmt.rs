mod support;
use self::support::*;
use lazy_static::lazy_static;
use std::io::Cursor;
use tracing::{self, subscriber::with_default, Level};
use tracing_subscriber::{filter::EnvFilter, fmt::time::FormatTime, fmt::MakeWriter, prelude::*};

use std::fmt;
use std::io;
use std::sync::{Mutex, MutexGuard, TryLockError};

struct MockTime;
impl FormatTime for MockTime {
    fn format_time(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "fake time")
    }
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

struct MockMakeWriter<'a> {
    buf: &'a Mutex<Vec<u8>>,
}

impl<'a> MockMakeWriter<'a> {
    fn new(buf: &'a Mutex<Vec<u8>>) -> Self {
        Self { buf }
    }
}

impl<'a> MakeWriter for MockMakeWriter<'a> {
    type Writer = MockWriter<'a>;

    fn make_writer(&self) -> Self::Writer {
        MockWriter::new(self.buf)
    }
}

#[cfg(feature = "ansi")]
#[test]
fn with_ansi_true() {
    lazy_static! {
        static ref BUF: Mutex<Vec<u8>> = Mutex::new(vec![]);
    }

    let make_writer = || MockWriter::new(&BUF);
    let expected = "\u{1b}[2mfake time\u{1b}[0m\u{1b}[32m INFO\u{1b}[0m ansi_fmt: some ansi test\n";
    test_ansi(make_writer, expected, true, &BUF);
}

#[cfg(feature = "ansi")]
#[test]
fn with_ansi_false() {
    lazy_static! {
        static ref BUF: Mutex<Vec<u8>> = Mutex::new(vec![]);
    }

    let make_writer = || MockWriter::new(&BUF);
    let expected = "fake time INFO ansi_fmt: some ansi test\n";

    test_ansi(make_writer, expected, false, &BUF);
}

#[cfg(not(feature = "ansi"))]
#[test]
fn without_ansi() {
    lazy_static! {
        static ref BUF: Mutex<Vec<u8>> = Mutex::new(vec![]);
    }

    let make_writer = || MockWriter::new(&BUF);
    let expected = "fake time INFO ansi_fmt: some ansi test\n";
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_writer(make_writer)
        .with_timer(MockTime)
        .finish();

    with_default(subscriber, || {
        tracing::info!("some ansi test");
    });

    let actual = String::from_utf8(BUF.try_lock().unwrap().to_vec()).unwrap();
    assert_eq!(expected, actual.as_str());
}

#[cfg(feature = "ansi")]
fn test_ansi<T>(make_writer: T, expected: &str, is_ansi: bool, buf: &Mutex<Vec<u8>>)
where
    T: MakeWriter + Send + Sync + 'static,
{
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_writer(make_writer)
        .with_ansi(is_ansi)
        .with_timer(MockTime)
        .finish();

    with_default(subscriber, || {
        tracing::info!("some ansi test");
    });

    let actual = String::from_utf8(buf.try_lock().unwrap().to_vec()).unwrap();
    assert_eq!(expected, actual.as_str());
}
