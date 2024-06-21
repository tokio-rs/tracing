//! A rolling file appender.
//!
//! Creates a new log file at a fixed frequency as defined by [`Rotation`].
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
    convert::TryFrom,
    fmt::{self, Debug},
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
};
use time::{format_description, Date, Duration, OffsetDateTime, Time};

mod builder;
pub use builder::{Builder, InitError};

/// A file appender with the ability to rotate log files at a fixed schedule.
///
/// `RollingFileAppender` implements the [`std:io::Write` trait][write] and will
/// block on write operations. It may be used with [`NonBlocking`] to perform
/// writes without blocking the current thread.
///
/// Additionally, `RollingFileAppender` also implements the [`MakeWriter`]
/// trait from `tracing-subscriber`, so it may also be used
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
pub struct RollingWriter<'a> {
    inner: RwLockReadGuard<'a, File>,
    current_size: &'a AtomicU64,
}

#[derive(Debug)]
struct Inner {
    log_directory: PathBuf,
    log_filename_prefix: Option<String>,
    log_filename_suffix: Option<String>,
    log_filename_index: Option<AtomicU64>,
    date_format: Vec<format_description::FormatItem<'static>>,
    rotation: Rotation,
    next_date: AtomicUsize,
    current_size: AtomicU64,
    max_files: Option<usize>,
}

// === impl RollingFileAppender ===

impl RollingFileAppender {
    /// Creates a new `RollingFileAppender`.
    ///
    /// A `RollingFileAppender` will have a fixed rotation whose frequency is
    /// defined by [`Rotation`]. The `directory` and
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
    /// Additional parameters can be configured using [`RollingFileAppender::builder`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn docs() {
    /// use tracing_appender::rolling::{RollingFileAppender, Rotation};
    /// let file_appender = RollingFileAppender::new(Rotation::HOURLY, "/some/directory", "prefix.log");
    /// # }
    /// ```
    pub fn new(
        rotation: Rotation,
        directory: impl AsRef<Path>,
        filename_prefix: impl AsRef<Path>,
    ) -> RollingFileAppender {
        let filename_prefix = filename_prefix
            .as_ref()
            .to_str()
            .expect("filename prefix must be a valid UTF-8 string");
        Self::builder()
            .rotation(rotation)
            .filename_prefix(filename_prefix)
            .build(directory)
            .expect("initializing rolling file appender failed")
    }

    /// Returns a new [`Builder`] for configuring a `RollingFileAppender`.
    ///
    /// The builder interface can be used to set additional configuration
    /// parameters when constructing a new appender.
    ///
    /// Unlike [`RollingFileAppender::new`], the [`Builder::build`] method
    /// returns a `Result` rather than panicking when the appender cannot be
    /// initialized. Therefore, the builder interface can also be used when
    /// appender initialization errors should be handled gracefully.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn docs() {
    /// use tracing_appender::rolling::{RollingFileAppender, Rotation};
    ///
    /// let file_appender = RollingFileAppender::builder()
    ///     .rotation(Rotation::HOURLY) // rotate log files once every hour
    ///     .filename_prefix("myapp") // log file names will be prefixed with `myapp.`
    ///     .filename_suffix("log") // log file names will be suffixed with `.log`
    ///     .build("/var/log") // try to build an appender that stores log files in `/var/log`
    ///     .expect("initializing rolling file appender failed");
    /// # drop(file_appender);
    /// # }
    /// ```
    #[must_use]
    pub fn builder() -> Builder {
        Builder::new()
    }

    fn from_builder(builder: &Builder, directory: impl AsRef<Path>) -> Result<Self, InitError> {
        Self::impl_from_builder(
            builder,
            directory,
            #[cfg(test)]
            Box::new(OffsetDateTime::now_utc),
        )
    }

    #[cfg(test)]
    fn from_builder_with_custom_now(
        builder: &Builder,
        directory: impl AsRef<Path>,
        now: Box<dyn Fn() -> OffsetDateTime + Send + Sync>,
    ) -> Result<Self, InitError> {
        Self::impl_from_builder(builder, directory, now)
    }

    fn impl_from_builder(
        builder: &Builder,
        directory: impl AsRef<Path>,
        #[cfg(test)] now: Box<dyn Fn() -> OffsetDateTime + Send + Sync>,
    ) -> Result<Self, InitError> {
        let Builder {
            rotation,
            prefix,
            suffix,
            max_files,
        } = &builder;

        let directory = directory.as_ref().to_path_buf();
        let (state, writer) = Inner::new(
            #[cfg(not(test))]
            OffsetDateTime::now_utc(),
            #[cfg(test)]
            now(),
            rotation.clone(),
            directory,
            prefix.clone(),
            suffix.clone(),
            *max_files,
        )?;

        Ok(Self {
            state,
            writer,
            #[cfg(test)]
            now,
        })
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
            let did_cas = self.state.advance_date_and_index(now, current_time);
            debug_assert!(did_cas, "if we have &mut access to the appender, no other thread can have advanced the timestamp...");
            self.state.refresh_writer(now, writer);
        }

        let written = writer.write(buf)?;

        self.state.current_size.fetch_add(
            u64::try_from(written).expect("usize to u64 conversion"),
            Ordering::SeqCst,
        );

        Ok(written)
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
            if self.state.advance_date_and_index(now, current_time) {
                self.state.refresh_writer(now, &mut self.writer.write());
            }
        }

        RollingWriter {
            inner: self.writer.read(),
            current_size: &self.state.current_size,
        }
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

/// Creates a minutely-rotating file appender. This will rotate the log file once per minute.
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
///     let collector = tracing_subscriber::fmt().with_writer(non_blocking_appender);
///
///     tracing::collect::with_default(collector.finish(), || {
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

/// Creates an hourly-rotating file appender.
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
///     let collector = tracing_subscriber::fmt().with_writer(non_blocking_appender);
///
///     tracing::collect::with_default(collector.finish(), || {
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

/// Creates a daily-rotating file appender.
///
/// The appender returned by `rolling::daily` can be used with `non_blocking` to create
/// a non-blocking, daily file appender.
///
/// A `RollingFileAppender` has a fixed rotation whose frequency is
/// defined by [`Rotation`]. The `directory` and
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
///     let collector = tracing_subscriber::fmt().with_writer(non_blocking_appender);
///
///     tracing::collect::with_default(collector.finish(), || {
///         tracing::event!(tracing::Level::INFO, "Hello");
///     });
/// # }
/// }
/// ```
///
/// This will result in a log file located at `/some/path/rolling.log.yyyy-MM-dd`.
pub fn daily(
    directory: impl AsRef<Path>,
    file_name_prefix: impl AsRef<Path>,
) -> RollingFileAppender {
    RollingFileAppender::new(Rotation::DAILY, directory, file_name_prefix)
}

/// Creates a non-rolling file appender.
///
/// The appender returned by `rolling::never` can be used with `non_blocking` to create
/// a non-blocking, non-rotating appender.
///
/// The location of the log file will be specified the `directory` passed in.
/// `file_name` specifies the complete name of the log file (no date or time is appended).
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
///     let collector = tracing_subscriber::fmt().with_writer(non_blocking_appender);
///
///     tracing::collect::with_default(collector.finish(), || {
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

/// Creates a file appender that rotates based on file size.
///
/// The appender returned by `rolling::max_size` can be used with `non_blocking` to create
/// a non-blocking, size-based file appender.
///
/// A `RollingFileAppender` has a fixed rotation whose frequency is
/// defined by [`Rotation`]. The `directory` and
/// `file_name_prefix` arguments determine the location and file name's _prefix_
/// of the log file. `RollingFileAppender` automatically appends the current date in UTC.
/// `max_number_of_bytes` specifies the maximum number of bytes for each file.
///
/// # Examples
///
/// ```rust
/// # #[clippy::allow(needless_doctest_main)]
/// fn main () {
/// # fn doc() {
///     let appender = tracing_appender::rolling::max_size("/some/path", "rolling.log", 100_000_000); // 100 MB
///     let (non_blocking_appender, _guard) = tracing_appender::non_blocking(appender);
///
///     let collector = tracing_subscriber::fmt().with_writer(non_blocking_appender);
///
///     tracing::collect::with_default(collector.finish(), || {
///         tracing::event!(tracing::Level::INFO, "Hello");
///     });
/// # }
/// }
/// ```
///
/// This will result in a log file located at `/some/path/rolling.log.yyyy-MM-dd-HH-mm-ss`.
pub fn max_size(
    directory: impl AsRef<Path>,
    file_name_prefix: impl AsRef<Path>,
    max_number_of_bytes: u64,
) -> RollingFileAppender {
    RollingFileAppender::new(
        Rotation::max_bytes(max_number_of_bytes),
        directory,
        file_name_prefix,
    )
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
pub struct Rotation {
    timed: Timed,
    max_bytes: Option<u64>,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
enum Timed {
    Minutely,
    Hourly,
    Daily,
    Never,
}

impl Rotation {
    /// Provides a minutely rotation.
    pub const MINUTELY: Self = Self {
        timed: Timed::Minutely,
        max_bytes: None,
    };

    /// Provides an hourly rotation.
    pub const HOURLY: Self = Self {
        timed: Timed::Hourly,
        max_bytes: None,
    };

    /// Provides a daily rotation.
    pub const DAILY: Self = Self {
        timed: Timed::Daily,
        max_bytes: None,
    };

    /// Provides a rotation that never rotates.
    pub const NEVER: Self = Self {
        timed: Timed::Never,
        max_bytes: None,
    };

    /// Provides a size-based rotation.
    pub const fn max_bytes(number_of_bytes: u64) -> Self {
        Self::NEVER.with_max_bytes(number_of_bytes)
    }

    /// Adds a maximum size limit to an existing `Rotation`.
    ///
    /// The new `Rotation` will rotate the log file when the log file reaches
    /// the specified size limit, or when the rotation period elapses, whichever
    /// occurs first.
    pub const fn with_max_bytes(mut self, number_of_bytes: u64) -> Self {
        self.max_bytes = Some(number_of_bytes);
        self
    }

    pub(crate) fn next_date(&self, current_date: &OffsetDateTime) -> Option<OffsetDateTime> {
        let unrounded_next_date = match self.timed {
            Timed::Minutely => *current_date + Duration::minutes(1),
            Timed::Hourly => *current_date + Duration::hours(1),
            Timed::Daily => *current_date + Duration::days(1),
            Timed::Never => return None,
        };
        Some(self.round_date(&unrounded_next_date))
    }

    // note that this method will panic if passed a `Timed::Never`.
    pub(crate) fn round_date(&self, date: &OffsetDateTime) -> OffsetDateTime {
        match self.timed {
            Timed::Minutely => {
                let time = Time::from_hms(date.hour(), date.minute(), 0)
                    .expect("Invalid time; this is a bug in tracing-appender");
                date.replace_time(time)
            }
            Timed::Hourly => {
                let time = Time::from_hms(date.hour(), 0, 0)
                    .expect("Invalid time; this is a bug in tracing-appender");
                date.replace_time(time)
            }
            Timed::Daily => {
                let time = Time::from_hms(0, 0, 0)
                    .expect("Invalid time; this is a bug in tracing-appender");
                date.replace_time(time)
            }
            // Timed::Never is impossible to round.
            Timed::Never => {
                unreachable!("Timed::Never is impossible to round.")
            }
        }
    }

    fn date_format(&self) -> Vec<format_description::FormatItem<'static>> {
        match self.timed {
            Timed::Minutely => format_description::parse("[year]-[month]-[day]-[hour]-[minute]"),
            Timed::Hourly => format_description::parse("[year]-[month]-[day]-[hour]"),
            Timed::Daily => format_description::parse("[year]-[month]-[day]"),
            Timed::Never => format_description::parse("[year]-[month]-[day]"),
        }
        .expect("Unable to create a formatter; this is a bug in tracing-appender")
    }
}

// === impl RollingWriter ===

impl io::Write for RollingWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = (&*self.inner).write(buf)?;
        self.current_size.fetch_add(
            u64::try_from(written).expect("usize to u64 conversion"),
            Ordering::SeqCst,
        );
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        (&*self.inner).flush()
    }
}

// === impl Inner ===

impl Inner {
    fn new(
        now: OffsetDateTime,
        rotation: Rotation,
        directory: impl AsRef<Path>,
        log_filename_prefix: Option<String>,
        log_filename_suffix: Option<String>,
        max_files: Option<usize>,
    ) -> Result<(Self, RwLock<File>), builder::InitError> {
        let log_directory = directory.as_ref().to_path_buf();
        let date_format = rotation.date_format();
        let next_date = rotation.next_date(&now);

        let mut inner = Inner {
            log_directory,
            log_filename_prefix,
            log_filename_suffix,
            log_filename_index: None,
            date_format,
            next_date: AtomicUsize::new(
                next_date
                    .map(|date| date.unix_timestamp() as usize)
                    .unwrap_or(0),
            ),
            rotation,
            max_files,
            current_size: AtomicU64::new(0),
        };

        if let Some(max_bytes) = inner.rotation.max_bytes {
            let next_index = inner.find_filename_index(now, max_bytes);
            inner.log_filename_index = Some(AtomicU64::new(next_index));
        }

        let filename = inner.join_date(&now);
        let file = create_writer(inner.log_directory.as_ref(), &filename)?;

        let current_file_size = file
            .metadata()
            .map_err(builder::InitError::ctx("Failed to read file metadata"))?
            .len();
        inner.current_size = AtomicU64::new(current_file_size);

        let writer = RwLock::new(file);

        Ok((inner, writer))
    }

    pub(crate) fn join_date(&self, date: &OffsetDateTime) -> String {
        macro_rules! insert_dot {
            ($name:ident) => {
                if !$name.is_empty() {
                    $name.push_str(".");
                }
            };
        }

        let date = date
            .format(&self.date_format)
            .expect("Unable to format OffsetDateTime; this is a bug in tracing-appender");

        let mut filename = String::new();

        if let Some(prefix) = &self.log_filename_prefix {
            filename.push_str(prefix);
        };

        match self.rotation.timed {
            // Insert the date when there is time-based rotations
            Timed::Minutely | Timed::Hourly | Timed::Daily => {
                insert_dot!(filename);
                filename.push_str(&date);
            }
            // "Never" but no prefix and no suffix means we should use the date anyway
            // The date is always included when there is a filename index (size-based rotation)
            Timed::Never
                if (self.log_filename_prefix.is_none() && self.log_filename_suffix.is_none())
                    || self.log_filename_index.is_some() =>
            {
                insert_dot!(filename);
                filename.push_str(&date);
            }
            // Otherwise, the date must not be inserted
            Timed::Never => {}
        }

        if let Some(index) = &self.log_filename_index {
            insert_dot!(filename);
            filename.push_str(&index.load(Ordering::Acquire).to_string());
        }

        if let Some(suffix) = &self.log_filename_suffix {
            insert_dot!(filename);
            filename.push_str(suffix);
        }

        // Sanity check: we should never end up with an empty filename
        assert!(
            !filename.is_empty(),
            "log file name should never be empty; this is a bug in tracing-appender"
        );

        filename
    }

    fn filter_log_file(&self, entry: &fs::DirEntry) -> Option<FilteredLogFile> {
        let metadata = entry.metadata().ok()?;

        // the appender only creates files, not directories or symlinks,
        // so we should never delete a dir or symlink.
        if !metadata.is_file() {
            return None;
        }

        let filename = entry.file_name();

        // if the filename is not a UTF-8 string, skip it.
        let mut filename = filename.to_str()?;

        if let Some(prefix) = &self.log_filename_prefix {
            let striped = filename.strip_prefix(prefix)?;
            filename = striped.trim_start_matches('.');
        }

        if let Some(suffix) = &self.log_filename_suffix {
            let striped = filename.strip_suffix(suffix)?;
            filename = striped.trim_end_matches('.');
        }

        let mut found_index = None;

        let formatted_date = match (self.rotation.timed, filename.find('.')) {
            (Timed::Never, None)
                if self.log_filename_prefix.is_none() && self.log_filename_suffix.is_none() =>
            {
                let _date = Date::parse(filename, &self.date_format).ok()?;
                Some(filename.to_owned())
            }
            (_, Some(dot_idx)) => {
                // Check for <date>.<index> pattern

                let date_segment = &filename[..dot_idx];
                let index_segment = &filename[dot_idx + 1..];

                let _date = Date::parse(date_segment, &self.date_format).ok()?;
                let index = index_segment.parse::<u64>().ok()?;

                found_index = Some(index);
                Some(date_segment.to_owned())
            }
            (_, None) => {
                if Date::parse(filename, &self.date_format).is_ok() {
                    Some(filename.to_owned())
                } else {
                    None
                }
            }
        };

        if self.log_filename_prefix.is_none()
            && self.log_filename_suffix.is_none()
            && found_index.is_none()
            && formatted_date.is_none()
        {
            return None;
        }

        let created_at = metadata.created().ok()?;

        Some(FilteredLogFile {
            path: entry.path(),
            metadata,
            created_at,
            formatted_date,
            index: found_index,
        })
    }

    fn find_filename_index(&self, now: OffsetDateTime, max_bytes: u64) -> u64 {
        macro_rules! unwrap_or_zero {
            ($value:expr) => {
                if let Some(value) = $value {
                    value
                } else {
                    return 0;
                }
            };
        }

        let read_dir = unwrap_or_zero!(fs::read_dir(&self.log_directory).ok());

        let mut files: Vec<FilteredLogFile> = read_dir
            .filter_map(|entry| {
                let entry = entry.ok()?;
                self.filter_log_file(&entry)
            })
            .collect();

        files.sort_unstable_by_key(|file| file.created_at);

        let most_recent_file = unwrap_or_zero!(files.last());

        let most_recent_formatted_date = unwrap_or_zero!(&most_recent_file.formatted_date).clone();

        let formatted_date_now = now
            .format(&self.date_format)
            .expect("Unable to format OffsetDateTime; this is a bug in tracing-appender");

        if most_recent_formatted_date != formatted_date_now {
            return 0;
        }

        let latest_file = files
            .into_iter()
            .filter(|log_file| match &log_file.formatted_date {
                Some(formatted_date) => most_recent_formatted_date.eq(formatted_date),
                None => false,
            })
            .max_by_key(|log_file| log_file.index);

        let latest_file = unwrap_or_zero!(latest_file);

        let index = unwrap_or_zero!(latest_file.index);

        // Increase the index if the latest file was too big
        if latest_file.metadata.len() > max_bytes {
            index + 1
        } else {
            index
        }
    }

    fn prune_old_logs(&self, max_files: usize) {
        let files = fs::read_dir(&self.log_directory).map(|dir| {
            dir.filter_map(|entry| {
                let entry = entry.ok()?;
                self.filter_log_file(&entry)
            })
            .collect::<Vec<_>>()
        });

        let mut files = match files {
            Ok(files) => files,
            Err(error) => {
                eprintln!("Error reading the log directory/files: {}", error);
                return;
            }
        };
        if files.len() < max_files {
            return;
        }

        // sort the files by their creation timestamps.
        files.sort_by_key(|log_file| log_file.created_at);

        // delete files, so that (n-1) files remain, because we will create another log file
        for file in files.iter().take(files.len() - (max_files - 1)) {
            if let Err(error) = fs::remove_file(&file.path) {
                eprintln!(
                    "Failed to remove old log file {}: {}",
                    file.path.display(),
                    error
                );
            }
        }
    }

    fn refresh_writer(&self, now: OffsetDateTime, file: &mut File) {
        let filename = self.join_date(&now);

        if let Some(max_files) = self.max_files {
            self.prune_old_logs(max_files);
        }

        match create_writer(&self.log_directory, &filename) {
            Ok(new_file) => {
                if let Err(err) = file.flush() {
                    eprintln!("Couldn't flush previous writer: {}", err);
                }
                *file = new_file;
                self.current_size.store(0, Ordering::Release);
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

        // if there is a maximum size for the file, check that it is not exceeded
        match self.rotation.max_bytes {
            Some(max_bytes) if self.current_size.load(Ordering::Acquire) > max_bytes => {
                // maximum size is exceeded, the appender should rotate immediately
                return Some(next_date);
            }
            _ => {}
        }

        // otherwise, if the next date is 0, this appender *never* rotates log files.
        if next_date == 0 {
            return None;
        }

        if date.unix_timestamp() as usize >= next_date {
            return Some(next_date);
        }

        None
    }

    fn advance_date_and_index(&self, now: OffsetDateTime, current: usize) -> bool {
        let next_date = self
            .rotation
            .next_date(&now)
            .map(|date| date.unix_timestamp() as usize)
            .unwrap_or(0);
        let next_date_updated = self
            .next_date
            .compare_exchange(current, next_date, Ordering::AcqRel, Ordering::Acquire)
            .is_ok();

        match &self.log_filename_index {
            Some(index) if next_date_updated => {
                if current == next_date {
                    index.fetch_add(1, Ordering::SeqCst);
                } else {
                    index.store(0, Ordering::Release);
                }
            }
            _ => {}
        }

        next_date_updated
    }
}

fn create_writer(directory: &Path, filename: &str) -> Result<File, InitError> {
    let path = directory.join(filename);
    let mut open_options = OpenOptions::new();
    open_options.append(true).create(true);

    let new_file = open_options.open(path.as_path());
    if new_file.is_err() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(InitError::ctx("failed to create log directory"))?;
            return open_options
                .open(path)
                .map_err(InitError::ctx("failed to create initial log file"));
        }
    }

    new_file.map_err(InitError::ctx("failed to create initial log file"))
}

struct FilteredLogFile {
    path: PathBuf,
    metadata: fs::Metadata,
    created_at: std::time::SystemTime,
    /// This is the date found in the filename, rounded (as opposed to `created_at`)
    formatted_date: Option<String>,
    index: Option<u64>,
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
        expected = "internal error: entered unreachable code: Timed::Never is impossible to round."
    )]
    fn test_never_date_rounding() {
        let now = OffsetDateTime::now_utc();
        let _ = Rotation::NEVER.round_date(&now);
    }

    #[test]
    fn test_path_concatenation() {
        let format = format_description::parse(
            "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour \
         sign:mandatory]:[offset_minute]:[offset_second]",
        )
        .unwrap();
        let directory = tempfile::tempdir().expect("failed to create tempdir");

        let now = OffsetDateTime::parse("2020-02-01 10:01:00 +00:00:00", &format).unwrap();

        struct TestCase {
            expected: &'static str,
            rotation: Rotation,
            prefix: Option<&'static str>,
            suffix: Option<&'static str>,
        }

        let test = |TestCase {
                        expected,
                        rotation,
                        prefix,
                        suffix,
                    }| {
            let (inner, _) = Inner::new(
                now,
                rotation.clone(),
                directory.path(),
                prefix.map(ToString::to_string),
                suffix.map(ToString::to_string),
                None,
            )
            .unwrap();
            let path = inner.join_date(&now);
            assert_eq!(
                expected, path,
                "rotation = {:?}, prefix = {:?}, suffix = {:?}",
                rotation, prefix, suffix
            );
        };

        let test_cases = vec![
            // prefix only
            TestCase {
                expected: "app.log.2020-02-01-10-01.0",
                rotation: Rotation::MINUTELY.with_max_bytes(1024),
                prefix: Some("app.log"),
                suffix: None,
            },
            TestCase {
                expected: "app.log.2020-02-01-10.0",
                rotation: Rotation::HOURLY.with_max_bytes(1024),
                prefix: Some("app.log"),
                suffix: None,
            },
            TestCase {
                expected: "app.log.2020-02-01.0",
                rotation: Rotation::DAILY.with_max_bytes(1024),
                prefix: Some("app.log"),
                suffix: None,
            },
            TestCase {
                expected: "app.log.2020-02-01.0",
                rotation: Rotation::max_bytes(1024),
                prefix: Some("app.log"),
                suffix: None,
            },
            TestCase {
                expected: "app.log.2020-02-01-10-01",
                rotation: Rotation::MINUTELY,
                prefix: Some("app.log"),
                suffix: None,
            },
            TestCase {
                expected: "app.log.2020-02-01-10",
                rotation: Rotation::HOURLY,
                prefix: Some("app.log"),
                suffix: None,
            },
            TestCase {
                expected: "app.log.2020-02-01",
                rotation: Rotation::DAILY,
                prefix: Some("app.log"),
                suffix: None,
            },
            TestCase {
                expected: "app.log",
                rotation: Rotation::NEVER,
                prefix: Some("app.log"),
                suffix: None,
            },
            // prefix and suffix
            TestCase {
                expected: "app.2020-02-01-10-01.0.log",
                rotation: Rotation::MINUTELY.with_max_bytes(1024),
                prefix: Some("app"),
                suffix: Some("log"),
            },
            TestCase {
                expected: "app.2020-02-01-10.0.log",
                rotation: Rotation::HOURLY.with_max_bytes(1024),
                prefix: Some("app"),
                suffix: Some("log"),
            },
            TestCase {
                expected: "app.2020-02-01.0.log",
                rotation: Rotation::DAILY.with_max_bytes(1024),
                prefix: Some("app"),
                suffix: Some("log"),
            },
            TestCase {
                expected: "app.2020-02-01.0.log",
                rotation: Rotation::max_bytes(1024),
                prefix: Some("app"),
                suffix: Some("log"),
            },
            TestCase {
                expected: "app.2020-02-01-10-01.log",
                rotation: Rotation::MINUTELY,
                prefix: Some("app"),
                suffix: Some("log"),
            },
            TestCase {
                expected: "app.2020-02-01-10.log",
                rotation: Rotation::HOURLY,
                prefix: Some("app"),
                suffix: Some("log"),
            },
            TestCase {
                expected: "app.2020-02-01.log",
                rotation: Rotation::DAILY,
                prefix: Some("app"),
                suffix: Some("log"),
            },
            TestCase {
                expected: "app.log",
                rotation: Rotation::NEVER,
                prefix: Some("app"),
                suffix: Some("log"),
            },
            // suffix only
            TestCase {
                expected: "2020-02-01-10-01.0.log",
                rotation: Rotation::MINUTELY.with_max_bytes(1024),
                prefix: None,
                suffix: Some("log"),
            },
            TestCase {
                expected: "2020-02-01-10.0.log",
                rotation: Rotation::HOURLY.with_max_bytes(1024),
                prefix: None,
                suffix: Some("log"),
            },
            TestCase {
                expected: "2020-02-01.0.log",
                rotation: Rotation::DAILY.with_max_bytes(1024),
                prefix: None,
                suffix: Some("log"),
            },
            TestCase {
                expected: "2020-02-01.0.log",
                rotation: Rotation::max_bytes(1024),
                prefix: None,
                suffix: Some("log"),
            },
            TestCase {
                expected: "2020-02-01-10-01.log",
                rotation: Rotation::MINUTELY,
                prefix: None,
                suffix: Some("log"),
            },
            TestCase {
                expected: "2020-02-01-10.log",
                rotation: Rotation::HOURLY,
                prefix: None,
                suffix: Some("log"),
            },
            TestCase {
                expected: "2020-02-01.log",
                rotation: Rotation::DAILY,
                prefix: None,
                suffix: Some("log"),
            },
            TestCase {
                expected: "log",
                rotation: Rotation::NEVER,
                prefix: None,
                suffix: Some("log"),
            },
            // no suffix nor prefix
            TestCase {
                expected: "2020-02-01-10-01.0",
                rotation: Rotation::MINUTELY.with_max_bytes(1024),
                prefix: None,
                suffix: None,
            },
            TestCase {
                expected: "2020-02-01-10.0",
                rotation: Rotation::HOURLY.with_max_bytes(1024),
                prefix: None,
                suffix: None,
            },
            TestCase {
                expected: "2020-02-01.0",
                rotation: Rotation::DAILY.with_max_bytes(1024),
                prefix: None,
                suffix: None,
            },
            TestCase {
                expected: "2020-02-01.0",
                rotation: Rotation::max_bytes(1024),
                prefix: None,
                suffix: None,
            },
            TestCase {
                expected: "2020-02-01-10-01",
                rotation: Rotation::MINUTELY,
                prefix: None,
                suffix: None,
            },
            TestCase {
                expected: "2020-02-01-10",
                rotation: Rotation::HOURLY,
                prefix: None,
                suffix: None,
            },
            TestCase {
                expected: "2020-02-01",
                rotation: Rotation::DAILY,
                prefix: None,
                suffix: None,
            },
            TestCase {
                expected: "2020-02-01",
                rotation: Rotation::NEVER,
                prefix: None,
                suffix: None,
            },
        ];

        for test_case in test_cases {
            test(test_case)
        }
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
        let (state, writer) = Inner::new(
            now,
            Rotation::HOURLY,
            directory.path(),
            Some("test_make_writer".to_string()),
            None,
            None,
        )
        .unwrap();

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

    #[test]
    fn test_max_log_files() {
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::prelude::*;

        let format = format_description::parse(
            "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour \
         sign:mandatory]:[offset_minute]:[offset_second]",
        )
        .unwrap();

        let now = OffsetDateTime::parse("2020-02-01 10:01:00 +00:00:00", &format).unwrap();
        let directory = tempfile::tempdir().expect("failed to create tempdir");
        let (state, writer) = Inner::new(
            now,
            Rotation::HOURLY,
            &directory,
            Some("test_max_log_files".to_string()),
            None,
            Some(2),
        )
        .unwrap();

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

        // depending on the filesystem, the creation timestamp's resolution may
        // be as coarse as one second, so we need to wait a bit here to ensure
        // that the next file actually is newer than the old one.
        std::thread::sleep(std::time::Duration::from_secs(1));

        tracing::info!("file 2");

        // advance time by one second
        (*clock.lock().unwrap()) += Duration::seconds(1);

        tracing::info!("file 2");

        // advance time by one hour
        (*clock.lock().unwrap()) += Duration::hours(1);

        // again, sleep to ensure that the creation timestamps actually differ.
        std::thread::sleep(std::time::Duration::from_secs(1));

        tracing::info!("file 3");

        // advance time by one second
        (*clock.lock().unwrap()) += Duration::seconds(1);

        tracing::info!("file 3");

        drop(default);

        let dir_contents = fs::read_dir(&directory).expect("Failed to read directory");
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
                    panic!("this file should have been pruned already!");
                }
                "2020-02-01-11" => {
                    assert_eq!("file 2\nfile 2\n", file);
                }
                "2020-02-01-12" => {
                    assert_eq!("file 3\nfile 3\n", file);
                }
                x => panic!("unexpected date {}", x),
            }
        }
    }

    #[test]
    fn size_based_rotation() {
        let directory = tempfile::tempdir().expect("failed to create tempdir");

        let mut appender = RollingFileAppender::builder()
            .rotation(Rotation::max_bytes(8))
            .filename_prefix("size_based_rotation")
            .filename_suffix("log")
            .build_with_now(&directory, Box::new(|| OffsetDateTime::UNIX_EPOCH))
            .expect("initializing rolling file appender failed");

        writeln!(appender, "(file1) more than 8 bytes").unwrap();
        writeln!(appender, "(file2) more than 8 bytes again").unwrap();
        writeln!(appender, "(file3) and here is a third file").unwrap();

        let dir_contents = fs::read_dir(&directory).expect("read directory");
        println!("dir={:?}", dir_contents);

        let expected_files = [
            "size_based_rotation.1970-01-01.0.log",
            "size_based_rotation.1970-01-01.1.log",
            "size_based_rotation.1970-01-01.2.log",
        ];
        let mut expected_files: std::collections::HashSet<&str> =
            expected_files.iter().copied().collect();

        for entry in dir_contents {
            println!("entry={:?}", entry);

            let path = entry.expect("dir entry").path();
            let contents = fs::read_to_string(&path).expect("read file");
            println!("path={}\ncontents={:?}", path.display(), contents);

            let filename = path.file_name().unwrap().to_str().unwrap();
            expected_files
                .remove(filename)
                .then_some(())
                .expect("filename");
        }

        assert!(
            expected_files.is_empty(),
            "missing file(s): {:?}",
            expected_files
        );
    }
}
