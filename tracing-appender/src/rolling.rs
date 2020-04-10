use crate::inner::InnerAppender;
use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::{fs, io};
use tracing_subscriber::fmt::MakeWriter;

pub struct RollingFileAppender {
    inner: Mutex<InnerAppender<BufWriterFactory>>,
}

impl RollingFileAppender {
    fn writer(&self) -> RollingFileWriter {
        RollingFileWriter::new(&self.inner)
    }
}

pub struct RollingFileWriter<'a> {
    inner: &'a Mutex<InnerAppender<BufWriterFactory>>,
}

impl<'a> RollingFileWriter<'a> {
    fn new(inner: &'a Mutex<InnerAppender<BufWriterFactory>>) -> Self {
        Self { inner }
    }

    fn inner(&self) -> io::Result<MutexGuard<'a, InnerAppender<BufWriterFactory>>> {
        self.inner.lock().map_err(
            |_: std::sync::PoisonError<MutexGuard<InnerAppender<BufWriterFactory>>>| {
                io::Error::from(io::ErrorKind::Other)
            },
        )
    }
}

impl<'a> io::Write for RollingFileWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner()?.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner()?.flush()
    }
}

impl<'a> MakeWriter for RollingFileWriter<'a> {
    type Writer = RollingFileWriter<'a>;

    fn make_writer(&self) -> Self::Writer {
        RollingFileWriter::new(&self.inner)
    }
}

pub fn hourly<P: AsRef<Path>>(directory: P, file_name_prefix: P) -> RollingFileAppender {
    create_writer(
        Rotation::HOURLY,
        directory.as_ref(),
        file_name_prefix.as_ref(),
    )
}

pub fn daily<P: AsRef<Path>>(directory: P, file_name_prefix: P) -> RollingFileAppender {
    create_writer(
        Rotation::DAILY,
        directory.as_ref(),
        file_name_prefix.as_ref(),
    )
}

pub fn never<P: AsRef<Path>>(directory: P, file_name_prefix: P) -> RollingFileAppender {
    create_writer(
        Rotation::NEVER,
        directory.as_ref(),
        file_name_prefix.as_ref(),
    )
}

fn create_writer(
    rotation: Rotation,
    directory: &Path,
    file_name_prefix: &Path,
) -> RollingFileAppender {
    RollingFileAppender {
        inner: Mutex::new(
            InnerAppender::new(
                directory,
                file_name_prefix,
                rotation,
                BufWriterFactory {},
                Utc::now(),
            )
            .expect("Failed to create appender"),
        ),
    }
}

pub(crate) trait WriterFactory: Debug + Clone + Send {
    type W: Write + Debug + Send;

    fn create_writer(&self, directory: &str, filename: &str) -> io::Result<Self::W>;
}

#[derive(Clone, Debug)]
pub(crate) struct BufWriterFactory {}

impl WriterFactory for BufWriterFactory {
    type W = BufWriter<File>;

    fn create_writer(&self, directory: &str, filename: &str) -> io::Result<BufWriter<File>> {
        let file_path = Path::new(directory).join(filename);
        Ok(BufWriter::new(open_file_create_parent_dirs(&file_path)?))
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

#[derive(Clone, Eq, PartialEq, Debug)]
pub(crate) struct Rotation(RotationKind);

#[derive(Clone, Eq, PartialEq, Debug)]
enum RotationKind {
    Hourly,
    Daily,
    Never,
}

impl Rotation {
    pub const HOURLY: Self = Self(RotationKind::Hourly);
    pub const DAILY: Self = Self(RotationKind::Daily);
    pub const NEVER: Self = Self(RotationKind::Never);

    pub(crate) fn next_date(&self, current_date: &DateTime<Utc>) -> DateTime<Utc> {
        let unrounded_next_date = match *self {
            Rotation::HOURLY => *current_date + chrono::Duration::hours(1),
            Rotation::DAILY => *current_date + chrono::Duration::days(1),
            Rotation::NEVER => Utc.ymd(9999, 1, 1).and_hms(1, 0, 0),
        };
        self.round_date(&unrounded_next_date)
    }

    pub(crate) fn round_date(&self, date: &DateTime<Utc>) -> DateTime<Utc> {
        match *self {
            Rotation::HOURLY => {
                Utc.ymd(date.year(), date.month(), date.day())
                    .and_hms(date.hour(), 0, 0)
            }
            Rotation::DAILY => Utc
                .ymd(date.year(), date.month(), date.day())
                .and_hms(0, 0, 0),
            Rotation::NEVER => {
                Utc.ymd(date.year(), date.month(), date.day())
                    .and_hms(date.hour(), 0, 0)
            }
        }
    }

    pub(crate) fn join_date(&self, filename: &str, date: &DateTime<Utc>) -> String {
        match *self {
            Rotation::HOURLY => format!("{}.{}", filename, date.format("%F-%H")),
            Rotation::DAILY => format!("{}.{}", filename, date.format("%F")),
            Rotation::NEVER => filename.to_string(),
        }
    }
}
