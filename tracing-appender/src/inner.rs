use std::io;
use std::io::{Write, BufWriter};

use crate::rolling::{Rotation, BufWriterFactory};
use chrono::prelude::*;
use std::fmt::Debug;
use std::path::Path;
use std::fs::File;

#[derive(Debug)]
pub(crate) struct InnerAppender {
    log_directory: String,
    log_filename_prefix: String,
    writer: BufWriter<File>,
    writer_factory: BufWriterFactory,
    next_date: DateTime<Utc>,
    rotation: Rotation,
}

impl InnerAppender {
    fn write_with_ts(&mut self, buf: &[u8], date: DateTime<Utc>) -> io::Result<usize> {
        // Even if refresh_writer fails, we still have the original writer. Ignore errors
        // and proceed with the write.
        let _ = self.refresh_writer(date);
        self.writer.write(buf)
    }
}

impl io::Write for InnerAppender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let now = Utc::now();
        self.write_with_ts(buf, now)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl InnerAppender {
    pub(crate) fn new(
        log_directory: &Path,
        log_filename_prefix: &Path,
        rotation: Rotation,
        writer_factory: BufWriterFactory,
        now: DateTime<Utc>,
    ) -> io::Result<Self> {
        let log_directory = log_directory.to_str().unwrap();
        let log_filename_prefix = log_filename_prefix.to_str().unwrap();

        let filename = rotation.join_date(log_filename_prefix, &now);
        let next_date = rotation.next_date(&now);

        let mut appender = InnerAppender {
            log_directory: log_directory.to_string(),
            log_filename_prefix: log_filename_prefix.to_string(),
            writer: writer_factory.create_writer(log_directory, &filename)?,
            writer_factory,
            next_date,
            rotation,
        };

        appender
            .write_with_ts(b"Init Application\n", now)
            .and_then(|_| appender.flush().and(Ok(appender)))
    }
}

impl InnerAppender {
    pub(crate) fn refresh_writer(&mut self, now: DateTime<Utc>) {
        if self.should_rollover(now) {
            let filename = self.rotation.join_date(&self.log_filename_prefix, &now);

            self.next_date = self.rotation.next_date(&now);

            match self
                .writer_factory
                .create_writer(&self.log_directory, &filename)
            {
                Ok(writer) => self.writer = writer,
                Err(err) => eprintln!("Couldn't create writer for logs: {}", err),
            }
        }
    }

    pub(crate) fn should_rollover(&self, date: DateTime<Utc>) -> bool {
        date >= self.next_date
    }
}
