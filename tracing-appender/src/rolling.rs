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
//! - [`Rotation::never()`][never()]: This will result in log file located at `some_directory/log_file_name`
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
use crate::sync::{RwLock, RwLockReadGuard};
use std::{
    fmt::{self, Debug},
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};
use time::{format_description, Duration, OffsetDateTime, Time};

/// A file appender with the ability to rotate log files at a fixed schedule.
///
/// `RollingFileAppender` implements the [`std:io::Write` trait][write] and will
/// block on write operations. It may be used with [`NonBlocking`] to perform
/// writes without blocking the current thread.
///
/// Additionally, `RollingFileAppender` also implements the [`MakeWriter`]
/// trait from `tracing-appender`, so it may also be used
/// directly, without [`NonBlocking`].
///
/// [write]: std::io::Write
/// [`NonBlocking`]: super::non_blocking::NonBlocking
///
/// # Examples
///
/// Rolling a log file once every hour:
///
/// ```rust
/// # fn docs() {
/// let file_appender = tracing_appender::rolling::hourly("/some/directory", "prefix");
/// # }
/// ```
///
/// Combining a `RollingFileAppender` with another [`MakeWriter`] implementation:
///
/// ```rust
/// # fn docs() {
/// use tracing_subscriber::fmt::writer::MakeWriterExt;
///
/// // Log all events to a rolling log file.
/// let logfile = tracing_appender::rolling::hourly("/logs", "myapp-logs");

/// // Log `INFO` and above to stdout.
/// let stdout = std::io::stdout.with_max_level(tracing::Level::INFO);
///
/// tracing_subscriber::fmt()
///     // Combine the stdout and log file `MakeWriter`s into one
///     // `MakeWriter` that writes to both
///     .with_writer(stdout.and(logfile))
///     .init();
/// # }
/// ```
///
/// [`MakeWriter`]: tracing_subscriber::fmt::writer::MakeWriter
pub struct RollingFileAppender {
    state: Inner,
    writer: RwLock<File>,
    #[cfg(test)]
    now: Box<dyn Fn() -> OffsetDateTime + Send + Sync>,
}

/// A [writer] that writes to a rolling log file.
///
/// This is returned by the [`MakeWriter`] implementation for [`RollingFileAppender`].
///
/// [writer]: std::io::Write
/// [`MakeWriter`]: tracing_subscriber::fmt::writer::MakeWriter
#[derive(Debug)]
pub struct RollingWriter<'a>(RwLockReadGuard<'a, File>);

#[derive(Debug)]
struct Inner {
    log_directory: String,
    log_filename_prefix: String,
    rotation: Rotation,
    next_date: AtomicUsize,
}

// === impl RollingFileAppender ===

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
    /// - [`Rotation::never()`][never()]
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
        let now = OffsetDateTime::now_utc();
        let (state, writer) = Inner::new(now, rotation, directory, file_name_prefix);
        Self {
            state,
            writer,
            #[cfg(test)]
            now: Box::new(OffsetDateTime::now_utc),
        }
    }

    #[inline]
    fn now(&self) -> OffsetDateTime {
        #[cfg(test)]
        return (self.now)();

        #[cfg(not(test))]
        OffsetDateTime::now_utc()
    }
}

impl io::Write for RollingFileAppender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let now = self.now();
        let writer = self.writer.get_mut();
        if let Some(current_time) = self.state.should_rollover(now) {
            let _did_cas = self.state.advance_date(now, current_time);
            debug_assert!(_did_cas, "if we have &mut access to the appender, no other thread can have advanced the timestamp...");
            self.state.refresh_writer(now, writer);
        }
        writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.get_mut().flush()
    }
}

impl<'a> tracing_subscriber::fmt::writer::MakeWriter<'a> for RollingFileAppender {
    type Writer = RollingWriter<'a>;
    fn make_writer(&'a self) -> Self::Writer {
        let now = self.now();

        // Should we try to roll over the log file?
        if let Some(current_time) = self.state.should_rollover(now) {
            // Did we get the right to lock the file? If not, another thread
            // did it and we can just make a writer.
            if self.state.advance_date(now, current_time) {
                self.state.refresh_writer(now, &mut *self.writer.write());
            }
        }
        RollingWriter(self.writer.read())
    }
}

impl fmt::Debug for RollingFileAppender {
    // This manual impl is required because of the `now` field (only present
    // with `cfg(test)`), which is not `Debug`...
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RollingFileAppender")
            .field("state", &self.state)
            .field("writer", &self.writer)
            .finish()
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
/// ### Minutely Rotation
/// ```rust
/// # fn docs() {
/// use tracing_appender::rolling::Rotation;
/// let rotation = tracing_appender::rolling::Rotation::MINUTELY;
/// # }
/// ```
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

    pub(crate) fn next_date(&self, current_date: &OffsetDateTime) -> Option<OffsetDateTime> {
        let unrounded_next_date = match *self {
            Rotation::MINUTELY => *current_date + Duration::minutes(1),
            Rotation::HOURLY => *current_date + Duration::hours(1),
            Rotation::DAILY => *current_date + Duration::days(1),
            Rotation::NEVER => return None,
        };
        Some(self.round_date(&unrounded_next_date))
    }

    // note that this method will panic if passed a `Rotation::NEVER`.
    pub(crate) fn round_date(&self, date: &OffsetDateTime) -> OffsetDateTime {
        match *self {
            Rotation::MINUTELY => {
                let time = Time::from_hms(date.hour(), date.minute(), 0)
                    .expect("Invalid time; this is a bug in tracing-appender");
                date.replace_time(time)
            }
            Rotation::HOURLY => {
                let time = Time::from_hms(date.hour(), 0, 0)
                    .expect("Invalid time; this is a bug in tracing-appender");
                date.replace_time(time)
            }
            Rotation::DAILY => {
                let time = Time::from_hms(0, 0, 0)
                    .expect("Invalid time; this is a bug in tracing-appender");
                date.replace_time(time)
            }
            // Rotation::NEVER is impossible to round.
            Rotation::NEVER => {
                unreachable!("Rotation::NEVER is impossible to round.")
            }
        }
    }

    pub(crate) fn join_date(&self, filename: &str, date: &OffsetDateTime) -> String {
        match *self {
            Rotation::MINUTELY => {
                let format = format_description::parse("[year]-[month]-[day]-[hour]-[minute]")
                    .expect("Unable to create a formatter; this is a bug in tracing-appender");

                let date = date
                    .format(&format)
                    .expect("Unable to format OffsetDateTime; this is a bug in tracing-appender");
                format!("{}.{}", filename, date)
            }
            Rotation::HOURLY => {
                let format = format_description::parse("[year]-[month]-[day]-[hour]")
                    .expect("Unable to create a formatter; this is a bug in tracing-appender");

                let date = date
                    .format(&format)
                    .expect("Unable to format OffsetDateTime; this is a bug in tracing-appender");
                format!("{}.{}", filename, date)
            }
            Rotation::DAILY => {
                let format = format_description::parse("[year]-[month]-[day]")
                    .expect("Unable to create a formatter; this is a bug in tracing-appender");
                let date = date
                    .format(&format)
                    .expect("Unable to format OffsetDateTime; this is a bug in tracing-appender");
                format!("{}.{}", filename, date)
            }
            Rotation::NEVER => filename.to_string(),
        }
    }
}

// === impl RollingWriter ===

impl io::Write for RollingWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&*self.0).write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        (&*self.0).flush()
    }
}

// === impl Inner ===

impl Inner {
    fn new(
        now: OffsetDateTime,
        rotation: Rotation,
        directory: impl AsRef<Path>,
        file_name_prefix: impl AsRef<Path>,
    ) -> (Self, RwLock<File>) {
        let log_directory = directory.as_ref().to_str().unwrap();
        let log_filename_prefix = file_name_prefix.as_ref().to_str().unwrap();

        let filename = rotation.join_date(log_filename_prefix, &now);
        let next_date = rotation.next_date(&now);
        let writer = RwLock::new(
            create_writer(log_directory, &filename).expect("failed to create appender"),
        );

        let inner = Inner {
            log_directory: log_directory.to_string(),
            log_filename_prefix: log_filename_prefix.to_string(),
            next_date: AtomicUsize::new(
                next_date
                    .map(|date| date.unix_timestamp() as usize)
                    .unwrap_or(0),
            ),
            rotation,
        };
        (inner, writer)
    }

    fn refresh_writer(&self, now: OffsetDateTime, file: &mut File) {
        let filename = self.rotation.join_date(&self.log_filename_prefix, &now);

        match create_writer(&self.log_directory, &filename) {
            Ok(new_file) => {
                if let Err(err) = file.flush() {
                    eprintln!("Couldn't flush previous writer: {}", err);
                }
                *file = new_file;
            }
            Err(err) => eprintln!("Couldn't create writer for logs: {}", err),
        }
    }

    /// Checks whether or not it's time to roll over the log file.
    ///
    /// Rather than returning a `bool`, this returns the current value of
    /// `next_date` so that we can perform a `compare_exchange` operation with
    /// that value when setting the next rollover time.
    ///
    /// If this method returns `Some`, we should roll to a new log file.
    /// Otherwise, if this returns we should not rotate the log file.
    fn should_rollover(&self, date: OffsetDateTime) -> Option<usize> {
        let next_date = self.next_date.load(Ordering::Acquire);
        // if the next date is 0, this appender *never* rotates log files.
        if next_date == 0 {
            return None;
        }

        if date.unix_timestamp() as usize >= next_date {
            return Some(next_date);
        }

        None
    }

    fn advance_date(&self, now: OffsetDateTime, current: usize) -> bool {
        let next_date = self
            .rotation
            .next_date(&now)
            .map(|date| date.unix_timestamp() as usize)
            .unwrap_or(0);
        self.next_date
            .compare_exchange(current, next_date, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
}

fn create_writer(directory: &str, filename: &str) -> io::Result<File> {
    let path = Path::new(directory).join(filename);
    let mut open_options = OpenOptions::new();
    open_options.append(true).create(true);

    let new_file = open_options.open(path.as_path());
    if new_file.is_err() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            return open_options.open(path);
        }
    }

    new_file
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn find_str_in_log(dir_path: &Path, expected_value: &str) -> bool {
        let dir_contents = fs::read_dir(dir_path).expect("Failed to read directory");

        for entry in dir_contents {
            let path = entry.expect("Expected dir entry").path();
            let file = fs::read_to_string(&path).expect("Failed to read file");
            println!("path={}\nfile={:?}", path.display(), file);

            if file.as_str() == expected_value {
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

    fn test_appender(rotation: Rotation, file_prefix: &str) {
        let directory = tempfile::tempdir().expect("failed to create tempdir");
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
        test_appender(Rotation::HOURLY, "minutely.log");
    }

    #[test]
    fn write_hourly_log() {
        test_appender(Rotation::HOURLY, "hourly.log");
    }

    #[test]
    fn write_daily_log() {
        test_appender(Rotation::DAILY, "daily.log");
    }

    #[test]
    fn write_never_log() {
        test_appender(Rotation::NEVER, "never.log");
    }

    #[test]
    fn test_rotations() {
        // per-minute basis
        let now = OffsetDateTime::now_utc();
        let next = Rotation::MINUTELY.next_date(&now).unwrap();
        assert_eq!((now + Duration::MINUTE).minute(), next.minute());

        // per-hour basis
        let now = OffsetDateTime::now_utc();
        let next = Rotation::HOURLY.next_date(&now).unwrap();
        assert_eq!((now + Duration::HOUR).hour(), next.hour());

        // daily-basis
        let now = OffsetDateTime::now_utc();
        let next = Rotation::DAILY.next_date(&now).unwrap();
        assert_eq!((now + Duration::DAY).day(), next.day());

        // never
        let now = OffsetDateTime::now_utc();
        let next = Rotation::NEVER.next_date(&now);
        assert!(next.is_none());
    }

    #[test]
    #[should_panic(
        expected = "internal error: entered unreachable code: Rotation::NEVER is impossible to round."
    )]
    fn test_never_date_rounding() {
        let now = OffsetDateTime::now_utc();
        let _ = Rotation::NEVER.round_date(&now);
    }

    #[test]
    fn test_path_concatination() {
        let format = format_description::parse(
            "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour \
         sign:mandatory]:[offset_minute]:[offset_second]",
        )
        .unwrap();

        let now = OffsetDateTime::parse("2020-02-01 10:01:00 +00:00:00", &format).unwrap();

        // per-minute
        let path = Rotation::MINUTELY.join_date("app.log", &now);
        assert_eq!("app.log.2020-02-01-10-01", path);

        // per-hour
        let path = Rotation::HOURLY.join_date("app.log", &now);
        assert_eq!("app.log.2020-02-01-10", path);

        // per-day
        let path = Rotation::DAILY.join_date("app.log", &now);
        assert_eq!("app.log.2020-02-01", path);

        // never
        let path = Rotation::NEVER.join_date("app.log", &now);
        assert_eq!("app.log", path);
    }

    #[test]
    fn test_make_writer() {
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::prelude::*;

        let format = format_description::parse(
            "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour \
         sign:mandatory]:[offset_minute]:[offset_second]",
        )
        .unwrap();

        let now = OffsetDateTime::parse("2020-02-01 10:01:00 +00:00:00", &format).unwrap();
        let directory = tempfile::tempdir().expect("failed to create tempdir");
        let (state, writer) =
            Inner::new(now, Rotation::HOURLY, directory.path(), "test_make_writer");

        let clock = Arc::new(Mutex::new(now));
        let now = {
            let clock = clock.clone();
            Box::new(move || *clock.lock().unwrap())
        };
        let appender = RollingFileAppender { state, writer, now };
        let default = tracing_subscriber::fmt()
            .without_time()
            .with_level(false)
            .with_target(false)
            .with_max_level(tracing_subscriber::filter::LevelFilter::TRACE)
            .with_writer(appender)
            .finish()
            .set_default();

        tracing::info!("file 1");

        // advance time by one second
        (*clock.lock().unwrap()) += Duration::seconds(1);

        tracing::info!("file 1");

        // advance time by one hour
        (*clock.lock().unwrap()) += Duration::hours(1);

        tracing::info!("file 2");

        // advance time by one second
        (*clock.lock().unwrap()) += Duration::seconds(1);

        tracing::info!("file 2");

        drop(default);

        let dir_contents = fs::read_dir(directory.path()).expect("Failed to read directory");
        println!("dir={:?}", dir_contents);
        for entry in dir_contents {
            println!("entry={:?}", entry);
            let path = entry.expect("Expected dir entry").path();
            let file = fs::read_to_string(&path).expect("Failed to read file");
            println!("path={}\nfile={:?}", path.display(), file);

            match path
                .extension()
                .expect("found a file without a date!")
                .to_str()
                .expect("extension should be UTF8")
            {
                "2020-02-01-10" => {
                    assert_eq!("file 1\nfile 1\n", file);
                }
                "2020-02-01-11" => {
                    assert_eq!("file 2\nfile 2\n", file);
                }
                x => panic!("unexpected date {}", x),
            }
        }
    }
}
