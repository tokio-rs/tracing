use crate::inner::InnerAppender;
use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use std::fmt::Debug;
use std::io;
use std::path::Path;

/// A file appender with the ability to rotate log files at a fixed schedule.
///
/// The file appender has a fixed rotation, which is defined by
/// [`Rotation`]: tracing_appender/rolling/struct.Rotation.html
///
/// The log file output is determined by providing a directory and log file name prefix.
///
/// To construct a rolling file appender, use
/// [`RollingFileAppender::new()`]: tracing_appender/rolling/struct.RollingFileAppender.html#method.new
///
/// or use the provided helpers:
/// [`Rotation::hourly()`]: tracing_appender/rolling/fn.hourly.html
/// [`Rotation::daily()`]: tracing_appender/rolling/fn.daily.html
/// [`Rotation::never()`]: tracing_appender/rolling/fn.never.html
///
/// # Examples
///
/// ```rust
/// # fn docs() {
/// use tracing_appender::rolling::RollingFileAppender;
/// use tracing_appender::rolling::Rotation;
/// let file_appender = RollingFileAppender::new(Rotation::HOURLY, "/some/directory", "prefix.log");
/// # }
/// ```
#[derive(Debug)]
pub struct RollingFileAppender {
    inner: InnerAppender,
}

impl RollingFileAppender {
    /// Returns a `RollingFileAppender` that writes to a rolling log file in a provided `directory`
    /// and with a file name prefix.
    pub fn new(
        rotation: Rotation,
        directory: impl AsRef<Path>,
        file_name_prefix: impl AsRef<Path>,
    ) -> RollingFileAppender {
        RollingFileAppender {
            inner: InnerAppender::new(
                directory.as_ref(),
                file_name_prefix.as_ref(),
                rotation,
                Utc::now(),
            )
            .expect("Failed to create appender"),
        }
    }
}

impl io::Write for RollingFileAppender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Creates an hourly, rolling file appender.
///
/// The appender returned by `rolling::hourly` can be used with `non_blocking` to create
/// a non-blocking, hourly file appender.
///
/// The location of the log file will be specified the `directory` passed in.
/// `file_name_prefix` specifies the prefix of the log file. It is appended by the
/// current date and hour.
///
/// # Examples
///
/// ``` rust
/// # #[clippy::allow(needless_doctest_main)]
/// fn main () {
///
/// # fn doc() {
///     let appender = tracing_appender::rolling::hourly("/some/path", "rolling.log");
///     let (non_blocking_appender, _guard) = tracing_appender::non_blocking(appender);
///
///     let subscriber = tracing_subscriber::fmt().with_writer(non_blocking_appender);
///
///     tracing::subscriber::with_default(subscriber.finish(), || {
///         tracing::event!(tracing::Level::INFO, "Hello");
///     });
/// # }
/// }
/// ```
pub fn hourly(
    directory: impl AsRef<Path>,
    file_name_prefix: impl AsRef<Path>,
) -> RollingFileAppender {
    RollingFileAppender::new(Rotation::HOURLY, directory, file_name_prefix)
}

/// Creates an daily, rolling file appender.
///
/// The appender returned by `rolling::daily` can be used with `non_blocking` to create
/// a non-blocking, daily file appender.
///
/// The location of the log file will be specified the `directory` passed in.
/// `file_name_prefix` specifies the prefix of the log file. It is appended by the current date.
///
/// # Examples
///
/// ``` rust
/// # #[clippy::allow(needless_doctest_main)]
/// fn main () {
///
/// # fn doc() {
///     let appender = tracing_appender::rolling::daily("/some/path", "rolling.log");
///     let (non_blocking_appender, _guard) = tracing_appender::non_blocking(appender);
///
///     let subscriber = tracing_subscriber::fmt().with_writer(non_blocking_appender);
///
///     tracing::subscriber::with_default(subscriber.finish(), || {
///         tracing::event!(tracing::Level::INFO, "Hello");
///     });
/// # }
/// }
/// ```
pub fn daily(
    directory: impl AsRef<Path>,
    file_name_prefix: impl AsRef<Path>,
) -> RollingFileAppender {
    RollingFileAppender::new(Rotation::DAILY, directory, file_name_prefix)
}

/// Creates a non-rolling, file appender
///
/// The appender returned by `rolling::never` can be used with `non_blocking` to create
/// a non-blocking, appender that never rotates.
///
/// The location of the log file will be specified the `directory` passed in.
/// `file_name_prefix` specifies the prefix of the log file. No date is appended to it.
///
/// # Examples
///
/// ``` rust
/// # #[clippy::allow(needless_doctest_main)]
/// fn main () {
///
/// # fn doc() {
///     let appender = tracing_appender::rolling::never("/some/path", "non-rolling.log");
///     let (non_blocking_appender, _guard) = tracing_appender::non_blocking(appender);
///
///     let subscriber = tracing_subscriber::fmt().with_writer(non_blocking_appender);
///
///     tracing::subscriber::with_default(subscriber.finish(), || {
///         tracing::event!(tracing::Level::INFO, "Hello");
///     });
/// # }
/// }
/// ```
pub fn never(
    directory: impl AsRef<Path>,
    file_name_prefix: impl AsRef<Path>,
) -> RollingFileAppender {
    RollingFileAppender::new(Rotation::NEVER, directory, file_name_prefix)
}

/// `Rotation`s defines a fixed period for rolling of a log file.
///
/// To use a `Rotation`, you can do the following:
///
/// To get an hourly rotation:
/// ```rust
/// # fn docs() {
/// use tracing_appender::rolling::Rotation;
/// let rotation = tracing_appender::rolling::Rotation::HOURLY;
/// # }
/// ```
///
/// To get an daily rotation:
/// ```rust
/// # fn docs() {
/// use tracing_appender::rolling::Rotation;
/// let rotation = tracing_appender::rolling::Rotation::DAILY;
/// # }
/// ```
///
/// To never rotate:
/// ```rust
/// # fn docs() {
/// use tracing_appender::rolling::Rotation;
/// let rotation = tracing_appender::rolling::Rotation::NEVER;
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Rotation(RotationKind);

#[derive(Clone, Eq, PartialEq, Debug)]
enum RotationKind {
    Hourly,
    Daily,
    Never,
}

impl Rotation {
    /// Provides an hourly rotation
    pub const HOURLY: Self = Self(RotationKind::Hourly);
    /// Provides a daily rotation
    pub const DAILY: Self = Self(RotationKind::Daily);
    /// Provides a rotation that never rotates.
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
