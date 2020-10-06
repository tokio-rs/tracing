use std::{
    io,
    sync::{Mutex, MutexGuard},
};

use tracing_core::Dispatch;
use tracing_subscriber::FmtSubscriber;

/// A fake writer that writes into a buffer (behind a mutex).
pub struct MockWriter<'a> {
    buf: &'a Mutex<Vec<u8>>,
}

impl<'a> MockWriter<'a> {
    pub fn new(buf: &'a Mutex<Vec<u8>>) -> Self {
        Self { buf }
    }

    pub fn buf(&self) -> io::Result<MutexGuard<'a, Vec<u8>>> {
        // Note: The `lock` will block. This would be a problem in production code,
        // but is fine in tests.
        self.buf.lock().map_err(|_| io::Error::from(io::ErrorKind::Other))
    }
}

impl<'a> io::Write for MockWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Lock target buffer
        let mut target = self.buf()?;

        // Write to stdout in order to show up in tests
        print!("{}", String::from_utf8(buf.to_vec()).unwrap());

        // Write to buffer
        target.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buf()?.flush()
    }
}

/// Return a new subscriber that writes to the specified buffer.
pub fn get_subscriber(buf: &'static Mutex<Vec<u8>>, env_filter: &str) -> Dispatch {
    FmtSubscriber::builder()
        .with_env_filter(env_filter)
        .with_writer(move || MockWriter::new(buf))
        .with_level(true)
        .with_ansi(false)
        .into()
}
