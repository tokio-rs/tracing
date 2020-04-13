use std::io::{BufWriter, Write};
use std::{fs, io};

use crate::rolling::Rotation;
use chrono::prelude::*;
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::path::Path;

#[derive(Debug)]
pub(crate) struct InnerAppender {
    log_directory: String,
    log_filename_prefix: String,
    writer: BufWriter<File>,
    next_date: DateTime<Utc>,
    rotation: Rotation,
}

impl io::Write for InnerAppender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let now = Utc::now();
        self.write_timestamped(buf, now)
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
        now: DateTime<Utc>,
    ) -> io::Result<Self> {
        let log_directory = log_directory.to_str().unwrap();
        let log_filename_prefix = log_filename_prefix.to_str().unwrap();

        let filename = rotation.join_date(log_filename_prefix, &now);
        let next_date = rotation.next_date(&now);

        Ok(InnerAppender {
            log_directory: log_directory.to_string(),
            log_filename_prefix: log_filename_prefix.to_string(),
            writer: create_writer(log_directory, &filename)?,
            next_date,
            rotation,
        })
    }

    fn write_timestamped(&mut self, buf: &[u8], date: DateTime<Utc>) -> io::Result<usize> {
        // Even if refresh_writer fails, we still have the original writer. Ignore errors
        // and proceed with the write.
        let buf_len = buf.len();
        self.refresh_writer(date);
        self.writer.write_all(buf).map(|_| buf_len)
    }

    fn refresh_writer(&mut self, now: DateTime<Utc>) {
        if self.should_rollover(now) {
            let filename = self.rotation.join_date(&self.log_filename_prefix, &now);

            self.next_date = self.rotation.next_date(&now);

            match create_writer(&self.log_directory, &filename) {
                Ok(writer) => self.writer = writer,
                Err(err) => eprintln!("Couldn't create writer for logs: {}", err),
            }
        }
    }

    fn should_rollover(&self, date: DateTime<Utc>) -> bool {
        date >= self.next_date
    }
}

fn create_writer(directory: &str, filename: &str) -> io::Result<BufWriter<File>> {
    let file_path = Path::new(directory).join(filename);
    Ok(BufWriter::new(open_file_create_parent_dirs(&file_path)?))
}

fn open_file_create_parent_dirs(path: &Path) -> io::Result<File> {
    let mut open_options = OpenOptions::new();
    open_options.append(true).create(true);

    let new_file = open_options.open(path);
    if new_file.is_err() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            return open_options.open(path);
        }
    }

    new_file
}
