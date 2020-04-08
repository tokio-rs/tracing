use std::io::BufWriter;
use std::io::Write;
use std::{fs, io};

use crate::Rotation;
use chrono::prelude::*;
use std::fmt::Debug;
use std::fs::File;
use std::fs::OpenOptions;
use std::path::Path;

pub trait WriterFactory: Debug + Send {
    type W: Write + Debug + Send;

    fn create_writer(&self, directory: &str, filename: &str) -> io::Result<Self::W>;
}

#[derive(Debug)]
pub struct BufWriterFactory {}

impl WriterFactory for BufWriterFactory {
    type W = BufWriter<File>;

    fn create_writer(&self, directory: &str, filename: &str) -> io::Result<BufWriter<File>> {
        let filepath = Path::new(directory).join(filename);
        Ok(BufWriter::new(open_file_create_parent_dirs(&filepath)?))
    }
}

#[derive(Debug)]
pub struct InnerAppender<F: WriterFactory + Debug + Send> {
    log_directory: String,
    log_filename_prefix: String,
    writer: F::W,
    writer_factory: F,
    next_date: DateTime<Utc>,
    rotation: Rotation,
}

impl<F: WriterFactory> InnerAppender<F> {
    fn write_with_ts(&mut self, buf: &[u8], date: DateTime<Utc>) -> io::Result<usize> {
        // Even if refresh_writer fails, we still have the original writer. Ignore errors
        // and proceed with the write.
        let _ = self.refresh_writer(date);
        self.writer.write(buf)
    }
}

impl<F: WriterFactory> io::Write for InnerAppender<F> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let now = Utc::now();
        self.write_with_ts(buf, now)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<F: WriterFactory> InnerAppender<F> {
    pub fn new(
        log_directory: &str,
        log_filename_prefix: &str,
        rotation: Rotation,
        writer_factory: F,
        now: DateTime<Utc>,
    ) -> io::Result<Self> {
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

impl<F: WriterFactory> InnerAppender<F> {
    pub fn refresh_writer(&mut self, now: DateTime<Utc>) -> io::Result<()> {
        if self.should_rollover(now) {
            let filename = self.rotation.join_date(&self.log_filename_prefix, &now);

            self.next_date = self.rotation.next_date(&now);

            match self
                .writer_factory
                .create_writer(&self.log_directory, &filename)
            {
                Ok(writer) => {
                    self.writer = writer;
                    Ok(())
                }
                Err(err) => {
                    eprintln!("Couldn't create writer for logs: {}", err);
                    Err(err)
                }
            }
        } else {
            Ok(())
        }
    }

    pub fn should_rollover(&self, date: DateTime<Utc>) -> bool {
        date >= self.next_date
    }
}

// Open a file - if it throws any error, try creating the parent directory and then the file.
fn open_file_create_parent_dirs(path: &Path) -> io::Result<File> {
    let new_file = OpenOptions::new().append(true).create(true).open(path);
    if new_file.is_err() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            return OpenOptions::new().append(true).create(true).open(path);
        }
    }
    
    new_file
}
