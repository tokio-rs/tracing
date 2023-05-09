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
    fmt::{self, Debug},
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};
use time::{format_description, Date, Duration, OffsetDateTime, Time};

#[cfg(feature = "zipping")]
use io::Read;
#[cfg(feature = "zipping")]
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

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
pub struct RollingWriter<'a>(RwLockReadGuard<'a, File>);

#[derive(Debug)]
struct Inner {
    log_directory: PathBuf,
    log_filename_prefix: Option<String>,
    log_filename_suffix: Option<String>,
    date_format: Vec<format_description::FormatItem<'static>>,
    rotation: Rotation,
    next_date: AtomicUsize,
    max_files: Option<usize>,
    zipping: bool,
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
    ///     .max_log_files(5) // keep maximum 5 log files at a time
    ///     .zipping(true) // zip the finalized log files
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
        let Builder {
            ref rotation,
            ref prefix,
            ref suffix,
            ref max_files,
            ref zipping,
        } = builder;
        let directory = directory.as_ref().to_path_buf();
        let now = OffsetDateTime::now_utc();
        let (state, writer) = Inner::new(
            now,
            rotation.clone(),
            directory,
            prefix.clone(),
            suffix.clone(),
            *max_files,
            *zipping,
        )?;
        Ok(Self {
            state,
            writer,
            #[cfg(test)]
            now: Box::new(OffsetDateTime::now_utc),
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
                self.state.refresh_writer(now, &mut self.writer.write());
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

/// Creates a file appender that rotates daily.
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

    fn date_format(&self) -> Vec<format_description::FormatItem<'static>> {
        match *self {
            Rotation::MINUTELY => format_description::parse("[year]-[month]-[day]-[hour]-[minute]"),
            Rotation::HOURLY => format_description::parse("[year]-[month]-[day]-[hour]"),
            Rotation::DAILY => format_description::parse("[year]-[month]-[day]"),
            Rotation::NEVER => format_description::parse("[year]-[month]-[day]"),
        }
        .expect("Unable to create a formatter; this is a bug in tracing-appender")
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
        log_filename_prefix: Option<String>,
        log_filename_suffix: Option<String>,
        max_files: Option<usize>,
        zip: bool,
    ) -> Result<(Self, RwLock<File>), builder::InitError> {
        let log_directory = directory.as_ref().to_path_buf();
        let date_format = rotation.date_format();
        let next_date = rotation.next_date(&now);

        let inner = Inner {
            log_directory,
            log_filename_prefix,
            log_filename_suffix,
            date_format,
            next_date: AtomicUsize::new(
                next_date
                    .map(|date| date.unix_timestamp() as usize)
                    .unwrap_or(0),
            ),
            rotation,
            max_files,
            zipping: zip,
        };
        let filename = inner.join_date(&now);
        let writer = RwLock::new(create_writer(inner.log_directory.as_ref(), &filename)?);
        Ok((inner, writer))
    }

    pub(crate) fn join_date(&self, date: &OffsetDateTime) -> String {
        let date = date
            .format(&self.date_format)
            .expect("Unable to format OffsetDateTime; this is a bug in tracing-appender");

        match (
            &self.rotation,
            &self.log_filename_prefix,
            &self.log_filename_suffix,
        ) {
            (&Rotation::NEVER, Some(filename), None) => filename.to_string(),
            (&Rotation::NEVER, Some(filename), Some(suffix)) => format!("{}.{}", filename, suffix),
            (&Rotation::NEVER, None, Some(suffix)) => suffix.to_string(),
            (_, Some(filename), Some(suffix)) => format!("{}.{}.{}", filename, date, suffix),
            (_, Some(filename), None) => format!("{}.{}", filename, date),
            (_, None, Some(suffix)) => format!("{}.{}", date, suffix),
            (_, None, None) => date,
        }
    }

    // creating a custom error type should have been better for this function, but since across the library I haven't seen custom errors, but usage of `io::Result`,
    // I decided to follow the convention
    fn list_log_files(
        &self,
        include_zip_files: bool,
    ) -> io::Result<Vec<(fs::DirEntry, std::time::SystemTime)>> {
        let files = fs::read_dir(&self.log_directory)?
            .filter_map(|entry| {
                let entry = entry
                    .map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::Other,
                            format!("Error reading an entry from the log directory: {}", e),
                        )
                    })
                    .ok()?;

                let metadata = entry
                    .metadata()
                    .map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::Other,
                            format!("Error reading metadata of an entry: {}", e),
                        )
                    })
                    .ok()?;

                if !metadata.is_file() {
                    return None;
                }

                let filename = entry.file_name();
                let filename = filename
                    .to_str()
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::Other, "Filename is not a UTF-8 string")
                    })
                    .ok()?;

                if let Some(prefix) = &self.log_filename_prefix {
                    if !filename.starts_with(prefix) {
                        return None;
                    }
                }

                match (&self.zipping, &self.log_filename_suffix) {
                    (true, _) if !include_zip_files && filename.ends_with("zip") => {
                        return None;
                    }
                    (false, Some(suffix)) if !filename.ends_with(suffix) => {
                        return None;
                    }
                    _ => (),
                }

                if self.log_filename_prefix.is_none()
                    && self.log_filename_suffix.is_none()
                    && Date::parse(filename, &self.date_format).is_err()
                {
                    return None;
                }

                let created = metadata.created().ok()?;
                Some((entry, created))
            })
            .collect();

        Ok(files)
    }

    fn prune_old_logs(&self, max_files: usize) {
        // Read the log directory to find the log files
        let mut files = match self.list_log_files(true) {
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
        files.sort_by_key(|(_, created_at)| *created_at);

        // delete files, so that (n-1) files remain, because we will create another log file
        for (file, _) in files.iter().take(files.len() - (max_files - 1)) {
            if let Err(error) = fs::remove_file(file.path()) {
                eprintln!(
                    "Failed to remove old log file {}: {}",
                    file.path().display(),
                    error
                );
            }
        }
    }

    // This function is called after each refresh (rotating to a new file). Thus, it only zips the latest log file
    #[cfg(feature = "zipping")]
    fn zip_log(&self) -> io::Result<()> {
        // Read the log directory to find the log files
        let mut files = self
            .list_log_files(false)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

        // sort the files by their creation timestamps.
        files.sort_by_key(|(_, created_at)| *created_at);

        let (file, _) = files.last().unwrap();
        // Get input file path and read its content
        let input_file_path = file.path();
        let mut file_contents = Vec::new();
        let mut input_file = File::open(&input_file_path)?;
        input_file.read_to_end(&mut file_contents)?;

        // Add `.zip` extension to the filename
        let input_filename = input_file_path
            .file_name()
            .expect("Filename should be present")
            .to_str()
            .expect("Filename should be valid UTF-8");

        let output_filename = format!("{}.zip", input_filename);

        // Create a new ZIP archive and write the input file's content to it
        let output_file = File::create(&output_file_path)?;
        let mut zip_writer = ZipWriter::new(output_file);
        let file_options = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o755);

        // Add the input file to the ZIP archive
        let input_file_name = input_file_path.file_name().unwrap();
        zip_writer.start_file(input_file_name.to_string_lossy(), file_options)?;
        zip_writer.write_all(&file_contents)?;

        // Finish writing the ZIP archive
        zip_writer.finish()?;

        println!(
            "Successfully compressed {:?} to {:?}",
            input_file_path, output_file_path
        );
        std::fs::remove_file(&input_file_path)?;

        Ok(())
    }

    fn refresh_writer(&self, now: OffsetDateTime, file: &mut File) {
        let filename = self.join_date(&now);

        if let Some(max_files) = self.max_files {
            self.prune_old_logs(max_files);
        }

        // don't know how long this might take, should we start this in a thread?
        #[cfg(feature = "zipping")]
        if self.zipping {
            if let Err(err) = self.zip_log() {
                eprintln!("Couldn't successfully zip the latest log file: {}", err);
            }
        }

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
                false,
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
            false,
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
            directory.path(),
            Some("test_max_log_files".to_string()),
            None,
            Some(2),
            false,
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
}

#[cfg(feature = "zipping")]
#[test]
fn test_zipping() {
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::prelude::*;

    let format = format_description::parse(
        "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour \
     sign:mandatory]:[offset_minute]:[offset_second]",
    )
    .unwrap();

    let now = OffsetDateTime::parse("2023-05-01 10:01:00 +00:00:00", &format).unwrap();
    let directory = tempfile::tempdir().expect("failed to create tempdir");
    let (state, writer) = Inner::new(
        now,
        Rotation::HOURLY,
        directory.path(),
        Some("test_zip_log".to_string()),
        None,
        None,
        true,
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

    // advance time by one hour
    (*clock.lock().unwrap()) += Duration::hours(1);

    // depending on the filesystem, the creation timestamp's resolution may
    // be as coarse as one second, so we need to wait a bit here to ensure
    // that the next file actually is newer than the old one.
    std::thread::sleep(std::time::Duration::from_secs(1));

    tracing::info!("file 2");

    drop(default);

    let dir_contents = fs::read_dir(directory.path()).expect("Failed to read directory");
    println!("dir={:?}", dir_contents);

    let mut found_zip = false;
    let mut found_original = false;

    for entry in dir_contents {
        let entry = entry.expect("Expected dir entry");
        let path = entry.path();

        println!("entry={:?}", entry);

        if path.extension().unwrap() == "zip" {
            found_zip = true;
            let zip_file = fs::File::open(&path).expect("Failed to open zip file");
            let mut archive = zip::ZipArchive::new(zip_file).expect("Failed to read zip file");

            assert_eq!(
                archive.len(),
                1,
                "Expected exactly one file in the zip archive"
            );

            let mut file = archive
                .by_index(0)
                .expect("Failed to get the file from the archive");
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .expect("Failed to read the file content");

            assert_eq!(
                "file 1\n", contents,
                "Unexpected content in the zipped log file"
            );
        } else {
            let filename = path.file_name().unwrap().to_str().unwrap();
            if filename.starts_with("test_zip_log") && filename.ends_with("2023-05-01-10") {
                found_original = true;
            }
        }
    }

    assert!(
        found_zip,
        "Expected to find a zip file in the log directory"
    );
    assert!(!found_original, "Expected the first log file to be removed");
}

#[cfg(feature = "zipping")]
#[test]
fn test_integration_prune_and_zip() {
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::prelude::*;

    let format = format_description::parse(
        "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour \
     sign:mandatory]:[offset_minute]:[offset_second]",
    )
    .unwrap();

    let now = OffsetDateTime::parse("2023-05-01 10:01:00 +00:00:00", &format).unwrap();
    let directory = tempfile::tempdir().expect("failed to create tempdir");
    let (state, writer) = Inner::new(
        now,
        Rotation::HOURLY,
        directory.path(),
        Some("test_integration_prune_and_zip".to_string()),
        None,
        Some(2),
        true,
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

    for i in 0..5 {
        tracing::info!("file {}", i);

        // advance time by one hour
        (*clock.lock().unwrap()) += Duration::hours(1);

        // depending on the filesystem, the creation timestamp's resolution may
        // be as coarse as one second, so we need to wait a bit here to ensure
        // that the next file actually is newer than the old one.
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    drop(default);

    let dir_contents = fs::read_dir(directory.path()).expect("Failed to read directory");
    println!("dir={:?}", dir_contents);

    let mut zip_count = 0;
    let mut non_zip_count = 0;

    for entry in dir_contents {
        let entry = entry.expect("Expected dir entry");
        let path = entry.path();

        println!("entry={:?}", entry);

        if path.extension().unwrap() == "zip" {
            zip_count += 1;
            let zip_file = fs::File::open(&path).expect("Failed to open zip file");
            let archive = zip::ZipArchive::new(zip_file).expect("Failed to read zip file");

            assert_eq!(
                archive.len(),
                1,
                "Expected exactly one file in the zip archive"
            );
        } else {
            non_zip_count += 1;
        }
    }

    assert_eq!(
        zip_count, 1,
        "Expected to find exactly 1 zip files in the log directory"
    );

    assert_eq!(
        non_zip_count, 1,
        "Expected to find exactly 1 non-zip files in the log directory"
    );
}
