use std::io::{BufWriter, Write};
use std::{fs, io};

use crate::rolling::Rotation as Roll;
use crate::rotating::Rotation;

use chrono::prelude::*;
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::path::Path;

#[derive(Debug)]
pub(crate) struct InnerRollingAppender {
    log_directory: String,
    log_filename_prefix: String,
    writer: BufWriter<File>,
    next_date: DateTime<Utc>,
    roll: Roll,
}

impl io::Write for InnerRollingAppender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let now = Utc::now();
        self.write_timestamped(buf, now)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl InnerRollingAppender {
    pub(crate) fn new(
        log_directory: &Path,
        log_filename_prefix: &Path,
        roll: Roll,
        now: DateTime<Utc>,
    ) -> io::Result<Self> {
        let log_directory = log_directory.to_str().unwrap();
        let log_filename_prefix = log_filename_prefix.to_str().unwrap();

        let filename = roll.join_date(log_filename_prefix, &now);
        let next_date = roll.next_date(&now);

        Ok(InnerRollingAppender {
            log_directory: log_directory.to_string(),
            log_filename_prefix: log_filename_prefix.to_string(),
            writer: create_writer(log_directory, &filename)?,
            next_date,
            roll,
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
            let filename = self.roll.join_date(&self.log_filename_prefix, &now);

            self.next_date = self.roll.next_date(&now);

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

#[derive(Debug)]
pub(crate) struct InnerRotatingAppender {
    log_directory: String,
    log_filename_prefix: String,
    writer: BufWriter<File>,
    last_backup: usize,
    current_size: usize,
    rotation: Rotation,
}

impl io::Write for InnerRotatingAppender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let buf_len = buf.len();
        self.refresh_writer(&buf_len);
        self.writer.write_all(buf).map(|_| buf_len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl InnerRotatingAppender {
    pub(crate) fn new(
        rotation: Rotation,
        log_directory: &Path,
        log_filename_prefix: &Path,
    ) -> io::Result<Self> {
        let log_directory = log_directory.to_str().unwrap();
        let log_filename_prefix = log_filename_prefix.to_str().unwrap();
        let current_size = get_file_size(log_directory, log_filename_prefix)?;
        let last_backup = Self::find_last_backup(&rotation, log_directory, log_filename_prefix);
        Ok(Self {
            writer: create_writer(log_directory, log_filename_prefix)?,
            log_directory: log_directory.to_string(),
            log_filename_prefix: log_filename_prefix.to_string(),
            last_backup,
            current_size,
            rotation,
        })
    }
    fn refresh_writer(&mut self, size: &usize) {
        if self.rotation.should_rollover(self.current_size + size) {
            self.current_size = 0;
            if self.rotation.is_create_backup(self.last_backup) {
                self.last_backup += 1;
            }
            self.rotate_files();
            match create_writer(&self.log_directory, &self.log_filename_prefix) {
                Ok(writer) => self.writer = writer,
                Err(err) => eprintln!("Couldn't create writer for logs: {}", err),
            }
        }
        self.current_size += size;
    }
    fn rotate_files(&self) {
        for x in (1..=self.last_backup).rev() {
            let from = self.rotation.join_backup(&self.log_filename_prefix, x - 1);
            let to = self.rotation.join_backup(&self.log_filename_prefix, x);
            if let Err(err) = rename_file(&self.log_directory, &from, &to) {
                eprintln!("Couldn't rename backup log file: {}", err);
            }
        }
    }
    fn find_last_backup(
        rotation: &Rotation,
        log_directory: &str,
        log_filename_prefix: &str,
    ) -> usize {
        let mut last_backup = 0;
        while rotation.is_create_backup(last_backup) {
            let filename = rotation.join_backup(log_filename_prefix, last_backup + 1);
            if file_exist(log_directory, &filename) {
                last_backup += 1;
            } else {
                break;
            }
        }
        last_backup
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
fn get_file_size(directory: &str, filename: &str) -> io::Result<usize> {
    let file_path = Path::new(directory).join(filename);
    if file_path.exists() {
        Ok(std::fs::metadata(file_path)?.len() as usize)
    } else {
        Ok(0)
    }
}
fn file_exist(directory: &str, filename: &str) -> bool {
    let file_path = Path::new(directory).join(filename);
    file_path.as_path().exists()
}

fn rename_file(directory: &str, from: &str, to: &str) -> io::Result<()> {
    let from = Path::new(directory).join(from);
    let to = Path::new(directory).join(to);
    std::fs::rename(from, to)
}
