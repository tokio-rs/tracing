use crate::inner::InnerAppender;
use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use std::fmt::Debug;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::io;
use tracing_subscriber::fmt::MakeWriter;

pub struct RollingFileAppender {
    inner: Mutex<InnerAppender>,
}

impl RollingFileAppender {
    pub fn new(rotation: Rotation, directory: impl AsRef<Path>, file_name_prefix: impl AsRef<Path>) -> RollingFileAppender {
        RollingFileAppender {
            inner: Mutex::new(
                InnerAppender::new(
                    directory.as_ref(),
                    file_name_prefix.as_ref(),
                    rotation,
                    Utc::now(),
                )
                    .expect("Failed to create appender"),
            ),
        }
    }

    pub fn writer(&self) -> RollingFileWriter {
        RollingFileWriter::new(&self.inner)
    }
}

pub struct RollingFileWriter<'a> {
    inner: &'a Mutex<InnerAppender>,
}

impl<'a> RollingFileWriter<'a> {
    fn new(inner: &'a Mutex<InnerAppender>) -> Self {
        Self { inner }
    }

    fn inner(&self) -> io::Result<MutexGuard<'a, InnerAppender>> {
        self.inner.lock().map_err(
            |_: std::sync::PoisonError<MutexGuard<InnerAppender>>| {
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

pub fn hourly(directory: impl AsRef<Path>, file_name_prefix: impl AsRef<Path>) -> RollingFileAppender {
    RollingFileAppender::new(
        Rotation::HOURLY,
        directory,
        file_name_prefix,
    )
}

pub fn daily(directory: impl AsRef<Path>, file_name_prefix: impl AsRef<Path>) -> RollingFileAppender {
    RollingFileAppender::new(
        Rotation::DAILY,
        directory,
        file_name_prefix,
    )
}

pub fn never(directory: impl AsRef<Path>, file_name_prefix: impl AsRef<Path>) -> RollingFileAppender {
    RollingFileAppender::new(
        Rotation::NEVER,
        directory,
        file_name_prefix,
    )
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Rotation(RotationKind);

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
