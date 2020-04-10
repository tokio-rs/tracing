use std::{io, fs};
use std::io::{Write, BufWriter};

use crate::rolling::Rotation;
use chrono::prelude::*;
use std::fmt::Debug;
use std::path::Path;
use std::fs::{File, OpenOptions};

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

    fn write_with_ts(&mut self, buf: &[u8], date: DateTime<Utc>) -> io::Result<usize> {
        // Even if refresh_writer fails, we still have the original writer. Ignore errors
        // and proceed with the write.
        let _ = self.refresh_writer(date);
        self.writer.write(buf)
    }

    pub(crate) fn refresh_writer(&mut self, now: DateTime<Utc>) {
        if self.should_rollover(now) {
            let filename = self.rotation.join_date(&self.log_filename_prefix, &now);

            self.next_date = self.rotation.next_date(&now);

            match create_writer(&self.log_directory, &filename)
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


fn create_writer(directory: &str, filename: &str) -> io::Result<BufWriter<File>> {
    let file_path = Path::new(directory).join(filename);
    Ok(BufWriter::new(open_file_create_parent_dirs(&file_path)?))
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


