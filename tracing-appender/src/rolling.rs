//! A rolling file appender.
//!
//! Creates a new log file at a fixed frequency as defined by [`Rotation`](struct.Rotation.html).
//! Logs will be written to this file for the duration of the period and will automatically roll over
//! to the newly created log file once the time period has elapsed.
//!
//! The log file is created at the specified directory and file name prefix which *may* be appended with
//! the date and time.
//!
//! The following helpers are available for creating a rolling file appender.
//!
//! - [`Rotation::minutely()`][minutely]: A new log file in the format of `some_directory/log_file_name_prefix.yyyy-MM-dd-HH-mm`
//! will be created minutely (once per minute)
//! - [`Rotation::hourly()`][hourly]: A new log file in the format of `some_directory/log_file_name_prefix.yyyy-MM-dd-HH`
//! will be created hourly
//! - [`Rotation::daily()`][daily]: A new log file in the format of `some_directory/log_file_name_prefix.yyyy-MM-dd`
//! will be created daily
//! - [`Rotation::never()`][never]: This will result in log file located at `some_directory/log_file_name`
//!
//! [minutely]: fn.minutely.html
//! [hourly]: fn.hourly.html
//! [daily]: fn.daily.html
//! [never]: fn.never.html
//!
//! # Examples
//!
//! ```rust
//! # fn docs() {
//! use tracing_appender::rolling::{RollingFileAppender, Rotation};
//! let file_appender = RollingFileAppender::new(Rotation::HOURLY, "/some/directory", "prefix.log");
//! # }
//! ```
use crate::inner::InnerAppender;
use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use std::ffi::{OsStr, OsString};
use std::fmt::Debug;
use std::io;
use std::path::{Path, PathBuf};

/// A file appender with the ability to rotate log files at a fixed schedule.
///
/// `RollingFileAppender` implements [`std:io::Write` trait][write] and will block on write operations.
/// It may be used with [`NonBlocking`][non-blocking] to perform writes without
/// blocking the current thread.
///
/// [write]: https://doc.rust-lang.org/nightly/std/io/trait.Write.html
/// [non-blocking]: ../non_blocking/struct.NonBlocking.html
///
/// # Examples
///
/// ```rust
/// # fn docs() {
/// let file_appender = tracing_appender::rolling::hourly("/some/directory", "prefix.log");
/// # }
/// ```
#[derive(Debug)]
pub struct RollingFileAppender {
    inner: InnerAppender,
}

impl RollingFileAppender {
    /// Creates a new `RollingFileAppender`.
    ///
    /// A `RollingFileAppender` will have a fixed rotation whose frequency is
    /// defined by [`Rotation`](struct.Rotation.html). The `directory` and
    /// `file_name_prefix` arguments determine the location and file name's _prefix_
    /// of the log file. `RollingFileAppender` will automatically append the current date
    /// and hour (UTC format) to the file name.
    ///
    /// Alternatively, a `RollingFileAppender` can be constructed using one of the following helpers:
    ///
    /// - [`Rotation::minutely()`][minutely],
    /// - [`Rotation::hourly()`][hourly],
    /// - [`Rotation::daily()`][daily],
    /// - [`Rotation::never()`][never]
    ///
    /// [minutely]: fn.minutely.html
    /// [hourly]: fn.hourly.html
    /// [daily]: fn.daily.html
    /// [never]: fn.never.html
    ///
    /// # Examples
    /// ```rust
    /// # fn docs() {
    /// use tracing_appender::rolling::{RollingFileAppender, Rotation};
    /// let file_appender = RollingFileAppender::new(Rotation::HOURLY, "/some/directory", "prefix.log");
    /// # }
    /// ```
    pub fn new(
        rotation: Rotation,
        directory: impl AsRef<Path>,
        file_name_prefix: impl AsRef<Path>,
    ) -> RollingFileAppender {
        let path = file_name_prefix.as_ref();
        let template = prefixed(directory, path.as_os_str());
        RollingFileAppender::with_custom_template(rotation, template)
    }

    /// Create a new [`RollingFileAppender`] that uses a custom template for
    /// generating log files.
    pub fn with_custom_template<T>(rotation: Rotation, template: T) -> RollingFileAppender
    where
        T: FilenameTemplate,
    {
        RollingFileAppender {
            inner: InnerAppender::new(template, rotation, Utc::now())
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

/// Creates a minutely, rolling file appender. This will rotate the log file once per minute.
///
/// The appender returned by `rolling::minutely` can be used with `non_blocking` to create
/// a non-blocking, minutely file appender.
///
/// The directory of the log file is specified with the `directory` argument.
/// `file_name_prefix` specifies the _prefix_ of the log file. `RollingFileAppender`
/// adds the current date, hour, and minute to the log file in UTC.
///
/// # Examples
///
/// ``` rust
/// # #[clippy::allow(needless_doctest_main)]
/// fn main () {
/// # fn doc() {
///     let appender = tracing_appender::rolling::minutely("/some/path", "rolling.log");
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
///
/// This will result in a log file located at `/some/path/rolling.log.yyyy-MM-dd-HH-mm`.
pub fn minutely(
    directory: impl AsRef<Path>,
    file_name_prefix: impl AsRef<Path>,
) -> RollingFileAppender {
    RollingFileAppender::new(Rotation::MINUTELY, directory, file_name_prefix)
}

/// Creates an hourly, rolling file appender.
///
/// The appender returned by `rolling::hourly` can be used with `non_blocking` to create
/// a non-blocking, hourly file appender.
///
/// The directory of the log file is specified with the `directory` argument.
/// `file_name_prefix` specifies the _prefix_ of the log file. `RollingFileAppender`
/// adds the current date and hour to the log file in UTC.
///
/// # Examples
///
/// ``` rust
/// # #[clippy::allow(needless_doctest_main)]
/// fn main () {
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
///
/// This will result in a log file located at `/some/path/rolling.log.yyyy-MM-dd-HH`.
pub fn hourly(
    directory: impl AsRef<Path>,
    file_name_prefix: impl AsRef<Path>,
) -> RollingFileAppender {
    RollingFileAppender::new(Rotation::HOURLY, directory, file_name_prefix)
}

/// Creates a file appender that rotates daily.
///
/// The appender returned by `rolling::daily` can be used with `non_blocking` to create
/// a non-blocking, daily file appender.
///
/// A `RollingFileAppender` has a fixed rotation whose frequency is
/// defined by [`Rotation`](struct.Rotation.html). The `directory` and
/// `file_name_prefix` arguments determine the location and file name's _prefix_
/// of the log file. `RollingFileAppender` automatically appends the current date in UTC.
///
/// # Examples
///
/// ``` rust
/// # #[clippy::allow(needless_doctest_main)]
/// fn main () {
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
///
/// This will result in a log file located at `/some/path/rolling.log.yyyy-MM-dd-HH`.
pub fn daily(
    directory: impl AsRef<Path>,
    file_name_prefix: impl AsRef<Path>,
) -> RollingFileAppender {
    RollingFileAppender::new(Rotation::DAILY, directory, file_name_prefix)
}

/// Creates a non-rolling, file appender
///
/// The appender returned by `rolling::never` can be used with `non_blocking` to create
/// a non-blocking, non-rotating appender.
///
/// The location of the log file will be specified the `directory` passed in.
/// `file_name` specifies the prefix of the log file. No date or time is appended.
///
/// # Examples
///
/// ``` rust
/// # #[clippy::allow(needless_doctest_main)]
/// fn main () {
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
///
/// This will result in a log file located at `/some/path/non-rolling.log`.
pub fn never(directory: impl AsRef<Path>, file_name: impl AsRef<Path>) -> RollingFileAppender {
    RollingFileAppender::new(Rotation::NEVER, directory, file_name)
}

/// Defines a fixed period for rolling of a log file.
///
/// To use a `Rotation`, pick one of the following options:
///
/// ### Hourly Rotation
/// ```rust
/// # fn docs() {
/// use tracing_appender::rolling::Rotation;
/// let rotation = tracing_appender::rolling::Rotation::HOURLY;
/// # }
/// ```
///
/// ### Daily Rotation
/// ```rust
/// # fn docs() {
/// use tracing_appender::rolling::Rotation;
/// let rotation = tracing_appender::rolling::Rotation::DAILY;
/// # }
/// ```
///
/// ### No Rotation
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
    Minutely,
    Hourly,
    Daily,
    Never,
}

impl Rotation {
    /// Provides an minutely rotation
    pub const MINUTELY: Self = Self(RotationKind::Minutely);
    /// Provides an hourly rotation
    pub const HOURLY: Self = Self(RotationKind::Hourly);
    /// Provides a daily rotation
    pub const DAILY: Self = Self(RotationKind::Daily);
    /// Provides a rotation that never rotates.
    pub const NEVER: Self = Self(RotationKind::Never);

    pub(crate) fn next_date(&self, current_date: &DateTime<Utc>) -> DateTime<Utc> {
        let unrounded_next_date = match *self {
            Rotation::MINUTELY => *current_date + chrono::Duration::minutes(1),
            Rotation::HOURLY => *current_date + chrono::Duration::hours(1),
            Rotation::DAILY => *current_date + chrono::Duration::days(1),
            Rotation::NEVER => Utc.ymd(9999, 1, 1).and_hms(1, 0, 0),
        };
        self.round_date(&unrounded_next_date)
    }

    pub(crate) fn round_date(&self, date: &DateTime<Utc>) -> DateTime<Utc> {
        match *self {
            Rotation::MINUTELY => Utc.ymd(date.year(), date.month(), date.day()).and_hms(
                date.hour(),
                date.minute(),
                0,
            ),
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
}

/// A template that can be used to get the name of the next log file.
pub trait FilenameTemplate: Send + Sync + 'static {
    /// Get the filename that corresponds to this timestamp.
    fn next_log_file(&mut self, date: &DateTime<Utc>, rotation: &Rotation) -> PathBuf;
}

impl<F> FilenameTemplate for F
where
    F: FnMut(&DateTime<Utc>, &Rotation) -> PathBuf + Send + Sync + 'static,
{
    fn next_log_file(&mut self, date: &DateTime<Utc>, rotation: &Rotation) -> PathBuf {
        (self)(date, rotation)
    }
}

impl FilenameTemplate for Box<dyn FilenameTemplate + Send + Sync + 'static> {
    fn next_log_file(&mut self, date: &DateTime<Utc>, rotation: &Rotation) -> PathBuf {
        (**self).next_log_file(date, rotation)
    }
}

/// Get a [`FilenameTemplate`] with a timestamp appended to a common prefix.
pub fn prefixed<D, P>(log_directory: D, prefix: P) -> Prefixed
where
    D: AsRef<Path>,
    P: AsRef<OsStr>,
{
    Prefixed::new(log_directory, prefix)
}

/// A [`FilenameTemplate`] with a timestamp appended to a common prefix.
#[derive(Debug, Clone, PartialEq)]
pub struct Prefixed {
    log_directory: PathBuf,
    prefix: OsString,
}

impl Prefixed {
    /// Create a new [`Prefixed`] `FileTemplate`.
    pub fn new<D, P>(log_directory: D, prefix: P) -> Self
    where
        D: AsRef<Path>,
        P: AsRef<OsStr>,
    {
        Prefixed {
            log_directory: log_directory.as_ref().to_owned(),
            prefix: prefix.as_ref().to_owned(),
        }
    }
}

impl FilenameTemplate for Prefixed {
    fn next_log_file(&mut self, date: &DateTime<Utc>, rotation: &Rotation) -> PathBuf {
        let mut last_segment = self.prefix.clone();

        if let Some(timestamp) = format_date_for_rotation(date, rotation) {
            last_segment.push(".");
            last_segment.push(&timestamp);
        }

        self.log_directory.join(last_segment)
    }
}

/// Create a new [`FilenameTemplate`] which uses a common file name and
/// extension.
///
/// This will generate a filename like `/var/log/MyApplication.2020-02-01.log`.
pub fn with_name_and_extension<D, N, E>(
    log_directory: D,
    name: N,
    extension: E,
) -> WithNameAndExtension
where
    D: AsRef<Path>,
    N: AsRef<OsStr>,
    E: AsRef<OsStr>,
{
    WithNameAndExtension::new(log_directory, name, extension)
}

/// A [`FilenameTemplate`] with a timestamp appended to a common prefix.
#[derive(Debug, Clone, PartialEq)]
pub struct WithNameAndExtension {
    log_directory: PathBuf,
    name: OsString,
    extension: OsString,
}

impl WithNameAndExtension {
    /// Create a new [`WithNameAndExtension`] `FileTemplate`.
    pub fn new<D, N, E>(log_directory: D, name: N, extension: E) -> Self
    where
        D: AsRef<Path>,
        N: AsRef<OsStr>,
        E: AsRef<OsStr>,
    {
        WithNameAndExtension {
            log_directory: log_directory.as_ref().to_owned(),
            name: name.as_ref().to_owned(),
            extension: extension.as_ref().to_owned(),
        }
    }
}

impl FilenameTemplate for WithNameAndExtension {
    fn next_log_file(&mut self, date: &DateTime<Utc>, rotation: &Rotation) -> PathBuf {
        let mut last_segment = self.name.clone();

        if let Some(timestamp) = format_date_for_rotation(date, rotation) {
            last_segment.push(".");
            last_segment.push(&timestamp);
        }

        last_segment.push(".");
        last_segment.push(&self.extension);

        self.log_directory.join(last_segment)
    }
}

fn format_date_for_rotation(date: &DateTime<Utc>, rotation: &Rotation) -> Option<String> {
    match *rotation {
        Rotation::MINUTELY => Some(date.format("%F-%H-%M").to_string()),
        Rotation::HOURLY => Some(date.format("%F-%H").to_string()),
        Rotation::DAILY => Some(date.format("%F").to_string()),
        Rotation::NEVER => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempdir::TempDir;

    fn find_str_in_log(dir_path: &Path, expected_value: &str) -> bool {
        let dir_contents = fs::read_dir(dir_path).expect("Failed to read directory");

        for entry in dir_contents {
            let path = entry.expect("Expected dir entry").path();
            let result = fs::read_to_string(path).expect("Failed to read file");

            if result.as_str() == expected_value {
                return true;
            }
        }

        false
    }

    fn write_to_log(appender: &mut RollingFileAppender, msg: &str) {
        appender
            .write_all(msg.as_bytes())
            .expect("Failed to write to appender");
        appender.flush().expect("Failed to flush!");
    }

    fn test_appender(rotation: Rotation, directory: TempDir, file_prefix: &str) {
        let mut appender = RollingFileAppender::new(rotation, directory.path(), file_prefix);

        let expected_value = "Hello";
        write_to_log(&mut appender, expected_value);
        assert!(find_str_in_log(directory.path(), expected_value));

        directory
            .close()
            .expect("Failed to explicitly close TempDir. TempDir should delete once out of scope.")
    }

    #[test]
    fn write_minutely_log() {
        test_appender(
            Rotation::HOURLY,
            TempDir::new("minutely").expect("Failed to create tempdir"),
            "minutely.log",
        );
    }

    #[test]
    fn write_hourly_log() {
        test_appender(
            Rotation::HOURLY,
            TempDir::new("hourly").expect("Failed to create tempdir"),
            "hourly.log",
        );
    }

    #[test]
    fn write_daily_log() {
        test_appender(
            Rotation::DAILY,
            TempDir::new("daily").expect("Failed to create tempdir"),
            "daily.log",
        );
    }

    #[test]
    fn write_never_log() {
        test_appender(
            Rotation::NEVER,
            TempDir::new("never").expect("Failed to create tempdir"),
            "never.log",
        );
    }

    #[test]
    fn test_next_date_minutely() {
        let r = Rotation::MINUTELY;

        let mock_now = Utc.ymd(2020, 2, 1).and_hms(0, 0, 0);
        let next = r.next_date(&mock_now);
        assert_eq!(mock_now.with_minute(1).unwrap(), next);

        let mock_now = Utc.ymd(2020, 2, 1).and_hms(0, 20, 30);
        let next = r.next_date(&mock_now);
        assert_eq!(
            mock_now
                .with_hour(0)
                .unwrap()
                .with_minute(21)
                .unwrap()
                .with_second(0)
                .unwrap(),
            next
        );

        let mock_now = Utc.ymd(2020, 2, 1).and_hms(0, 59, 0);
        let next = r.next_date(&mock_now);
        assert_eq!(mock_now.with_hour(1).unwrap().with_minute(0).unwrap(), next);

        let mock_now = Utc.ymd(2020, 2, 1).and_hms(23, 59, 0);
        let next = r.next_date(&mock_now);
        assert_eq!(
            mock_now
                .with_day(2)
                .unwrap()
                .with_hour(0)
                .unwrap()
                .with_minute(0)
                .unwrap(),
            next
        );

        let mock_now = Utc.ymd(2020, 12, 31).and_hms(23, 59, 0);
        let next = r.next_date(&mock_now);
        assert_eq!(Utc.ymd(2021, 1, 1).and_hms(0, 0, 0), next);
    }

    #[test]
    fn test_next_date_hourly() {
        let r = Rotation::HOURLY;

        let mock_now = Utc.ymd(2020, 2, 1).and_hms(0, 0, 0);
        let next = r.next_date(&mock_now);
        assert_eq!(mock_now.with_hour(1).unwrap(), next);

        let mock_now = Utc.ymd(2020, 2, 1).and_hms(0, 20, 0);
        let next = r.next_date(&mock_now);
        assert_eq!(mock_now.with_hour(1).unwrap().with_minute(0).unwrap(), next);

        let mock_now = Utc.ymd(2020, 2, 1).and_hms(1, 0, 0);
        let next = r.next_date(&mock_now);
        assert_eq!(mock_now.with_hour(2).unwrap(), next);

        let mock_now = Utc.ymd(2020, 2, 1).and_hms(23, 0, 0);
        let next = r.next_date(&mock_now);
        assert_eq!(mock_now.with_day(2).unwrap().with_hour(0).unwrap(), next);

        let mock_now = Utc.ymd(2020, 12, 31).and_hms(23, 0, 0);
        let next = r.next_date(&mock_now);
        assert_eq!(Utc.ymd(2021, 1, 1).and_hms(0, 0, 0), next);
    }

    #[test]
    fn test_next_date_daily() {
        let r = Rotation::DAILY;

        let mock_now = Utc.ymd(2020, 8, 1).and_hms(0, 0, 0);
        let next = r.next_date(&mock_now);
        assert_eq!(mock_now.with_day(2).unwrap().with_hour(0).unwrap(), next);

        let mock_now = Utc.ymd(2020, 8, 1).and_hms(0, 20, 5);
        let next = r.next_date(&mock_now);
        assert_eq!(Utc.ymd(2020, 8, 2).and_hms(0, 0, 0), next);

        let mock_now = Utc.ymd(2020, 8, 31).and_hms(11, 0, 0);
        let next = r.next_date(&mock_now);
        assert_eq!(Utc.ymd(2020, 9, 1).and_hms(0, 0, 0), next);

        let mock_now = Utc.ymd(2020, 12, 31).and_hms(23, 0, 0);
        let next = r.next_date(&mock_now);
        assert_eq!(Utc.ymd(2021, 1, 1).and_hms(0, 0, 0), next);
    }

    #[test]
    fn test_round_date_minutely() {
        let r = Rotation::MINUTELY;
        let mock_now = Utc.ymd(2020, 2, 1).and_hms(10, 3, 1);
        assert_eq!(
            Utc.ymd(2020, 2, 1).and_hms(10, 3, 0),
            r.round_date(&mock_now)
        );

        let mock_now = Utc.ymd(2020, 2, 1).and_hms(10, 3, 0);
        assert_eq!(mock_now, r.round_date(&mock_now));
    }

    #[test]
    fn test_round_date_hourly() {
        let r = Rotation::HOURLY;
        let mock_now = Utc.ymd(2020, 2, 1).and_hms(10, 3, 1);
        assert_eq!(
            Utc.ymd(2020, 2, 1).and_hms(10, 0, 0),
            r.round_date(&mock_now)
        );

        let mock_now = Utc.ymd(2020, 2, 1).and_hms(10, 0, 0);
        assert_eq!(mock_now, r.round_date(&mock_now));
    }

    #[test]
    fn test_rotation_path_minutely() {
        let mut template = prefixed("", "MyApplication.log");
        let r = Rotation::MINUTELY;
        let mock_now = Utc.ymd(2020, 2, 1).and_hms(10, 3, 1);
        let path = template.next_log_file(&mock_now, &r);
        assert_eq!(Path::new("MyApplication.log.2020-02-01-10-03"), path);
    }

    #[test]
    fn test_rotation_path_hourly() {
        let mut template = prefixed("", "MyApplication.log");
        let r = Rotation::HOURLY;
        let mock_now = Utc.ymd(2020, 2, 1).and_hms(10, 3, 1);
        let path = template.next_log_file(&mock_now, &r);
        assert_eq!(Path::new("MyApplication.log.2020-02-01-10"), path);
    }

    #[test]
    fn test_rotation_path_daily() {
        let mut template = prefixed("", "MyApplication.log");
        let r = Rotation::DAILY;
        let mock_now = Utc.ymd(2020, 2, 1).and_hms(10, 3, 1);
        let path = template.next_log_file(&mock_now, &r);
        assert_eq!(Path::new("MyApplication.log.2020-02-01"), path);
    }

    #[test]
    fn test_round_date_daily() {
        let r = Rotation::DAILY;
        let mock_now = Utc.ymd(2020, 2, 1).and_hms(10, 3, 1);
        assert_eq!(
            Utc.ymd(2020, 2, 1).and_hms(0, 0, 0),
            r.round_date(&mock_now)
        );

        let mock_now = Utc.ymd(2020, 2, 1).and_hms(0, 0, 0);
        assert_eq!(mock_now, r.round_date(&mock_now));
    }

    #[test]
    fn test_next_date_never() {
        let r = Rotation::NEVER;

        let mock_now = Utc.ymd(2020, 2, 1).and_hms(0, 0, 0);
        let next = r.next_date(&mock_now);
        assert_eq!(next, Utc.ymd(9999, 1, 1).and_hms(1, 0, 0));
    }

    #[test]
    fn test_join_date_never() {
        let mut template = prefixed("", "Hello.log");
        let r = Rotation::NEVER;

        let mock_now = Utc.ymd(2020, 2, 1).and_hms(0, 0, 0);
        let joined_date = template.next_log_file(&mock_now, &r);
        assert_eq!(joined_date, Path::new("Hello.log"));
    }

    #[test]
    fn daily_with_name_with_extension() {
        let mut template = with_name_and_extension("/var/log", "MyApplication", "log");

        let date = Utc.ymd(2020, 2, 1).and_hms(12, 20, 15);
        let rotation = Rotation::DAILY;

        let filename = template.next_log_file(&date, &rotation);
        assert_eq!(filename, Path::new("/var/log/MyApplication.2020-02-01.log"));
    }
}
