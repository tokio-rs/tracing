use std::io::{BufWriter, Write};
use std::{fs, io};

use crate::rolling::{FilenameTemplate, Rotation};
use chrono::prelude::*;
use std::fmt::{self, Debug, Formatter};
use std::fs::{File, OpenOptions};
use std::path::Path;

pub(crate) struct InnerAppender {
    template: Box<dyn FilenameTemplate>,
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

impl Debug for InnerAppender {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let InnerAppender {
            template: _,
            writer,
            next_date,
            rotation,
        } = self;

        f.debug_struct("InnerAppender")
            .field("writer", writer)
            .field("next_date", next_date)
            .field("rotation", rotation)
            .finish()
    }
}

impl InnerAppender {
    pub(crate) fn new<T>(template: T, rotation: Rotation, now: DateTime<Utc>) -> io::Result<Self>
    where
        T: FilenameTemplate + 'static,
    {
        let mut template = Box::new(template);

        let filename = template.next_log_file(&now, &rotation);
        let next_date = rotation.next_date(&now);

        Ok(InnerAppender {
            template,
            writer: create_writer(&filename)?,
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
            let filename = self.template.next_log_file(&now, &self.rotation);

            self.next_date = self.rotation.next_date(&now);

            match create_writer(&filename) {
                Ok(writer) => self.writer = writer,
                Err(err) => eprintln!("Couldn't create writer for logs: {}", err),
            }
        }
    }

    fn should_rollover(&self, date: DateTime<Utc>) -> bool {
        date >= self.next_date
    }
}

fn create_writer(filename: &Path) -> io::Result<BufWriter<File>> {
    Ok(BufWriter::new(open_file_create_parent_dirs(filename)?))
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
