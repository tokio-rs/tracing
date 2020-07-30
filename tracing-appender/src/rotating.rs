//! A rotating file appender.
//!
//! Creates a new log file every fixed number of bytes and a fixed number of backups defined by [`Rotation`](struct.Rotation.html).
//! Logs will be written to this file and will automatically rotate
//! to the newly created log file once the file has reached a ceratin size.
//! When a rotation occurs, if allowed old logs will be saved under a new name.
//!
//! When allowing backups the appender will save old log files by appending the extensions ‘.1’, ‘.2’ etc.,
//! to the filename.
//! For example, with a maximum backups allowed of 5 and a base file name of app.log,
//! you would get app.log, app.log.1, app.log.2, up to app.log.5.
//! The file being written to is always app.log.
//! When this file is filled, it is closed and renamed to app.log.1, and if files app.log.1, app.log.2, etc. exist,
//! then they are renamed to app.log.2, app.log.3 etc. respectively, app.log.5 will not be renamed to app.log.6.
//!
//! The log file is created at the specified directory and file name prefix which *may* be appended with
//! a backup index number.
//!
//! # Examples
//!
//! ```rust
//! # fn docs() {
//! use tracing_appender::rotating::{RotatingFileAppender, Rotation};
//! let file_appender = RotatingFileAppender::new(Rotation::mb_100(), "/some/directory", "prefix.log");
//! # }
//! ```
use crate::inner::InnerRotatingAppender;
use std::io;
use std::path::Path;

/// A file appender with the ability to rotate log files at a fixed size.
///
/// `RotatingFileAppender` implements [`std:io::Write` trait][write] and will block on write operations.
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
/// let file_appender = tracing_appender::rotating::mb_100("/some/directory", "prefix.log");
/// # }
/// ```
#[derive(Debug)]
pub struct RotatingFileAppender {
    inner: InnerRotatingAppender,
}

impl RotatingFileAppender {
    /// Creates a new `RotatingFileAppender`.
    ///
    /// A `RotatingFileAppender` will have a fixed rotation whose size is
    /// defined by [`Rotation`](struct.Rotation.html). The `directory` and
    /// `file_name_prefix` arguments determine the location and file name's _prefix_
    /// of the log file. `RotatingFileAppender` will add an index to old logs.
    ///
    /// Alternatively, a `RotatingFileAppender` can be constructed using one of the following helpers:
    ///
    /// - [`rotating::mb_100()`][mb_100],
    /// - [`rotating::never()`][never]
    ///
    /// [mb_100]: fn.mb_100.html
    /// [never]: fn.never.html
    ///
    /// # Examples
    /// ```rust
    /// # fn docs() {
    /// use tracing_appender::rotating::{RotatingFileAppender, Rotation};
    /// let file_appender = RotatingFileAppender::new(Rotation::mb_100(), "/some/directory", "prefix.log");
    /// # }
    /// ```
    pub fn new(
        rotation: Rotation,
        directory: impl AsRef<Path>,
        file_name_prefix: impl AsRef<Path>,
    ) -> RotatingFileAppender {
        RotatingFileAppender {
            inner: InnerRotatingAppender::new(
                rotation,
                directory.as_ref(),
                file_name_prefix.as_ref(),
            )
            .expect("Failed to create appender"),
        }
    }
}

impl io::Write for RotatingFileAppender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Creates an rotating file appender that will rotate after 100 MB and will allow 9 old log files.
///
/// The appender returned by `rotating::mb_100` can be used with `non_blocking` to create
/// a non-blocking, size based rotating file appender.
///
/// The directory of the log file is specified with the `directory` argument.
/// `file_name_prefix` specifies the _prefix_ of the log file. `RotatingFileAppender`
/// adds a backup index for old log files..
///
/// # Examples
///
/// ``` rust
/// # #[clippy::allow(needless_doctest_main)]
/// fn main () {
/// # fn doc() {
///     let appender = tracing_appender::rotating::mb_100("/some/path", "rotating.log");
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
/// This will result in a log file located at `/some/path/rotating.log`.
pub fn mb_100(
    directory: impl AsRef<Path>,
    file_name_prefix: impl AsRef<Path>,
) -> RotatingFileAppender {
    RotatingFileAppender::new(Rotation::mb_100(), directory, file_name_prefix)
}

/// Creates a non-rotating, file appender
///
/// The appender returned by `rotating::never` can be used with `non_blocking` to create
/// a non-blocking, non-rotating appender.
///
/// The location of the log file will be specified the `directory` passed in.
/// `file_name` specifies the prefix of the log file. No old log files will be provided.
///
/// # Examples
///
/// ``` rust
/// # #[clippy::allow(needless_doctest_main)]
/// fn main () {
/// # fn doc() {
///     let appender = tracing_appender::rotating::never("/some/path", "non-rotating.log");
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
/// This will result in a log file located at `/some/path/non-rotating.log`.
pub fn never(directory: impl AsRef<Path>, file_name: impl AsRef<Path>) -> RotatingFileAppender {
    RotatingFileAppender::new(Rotation::never(), directory, file_name)
}

/// Defines a fixed size and number of backups for rotating of a log file.
///
/// To use a `Rotation`, pick one of the following options use the [`new`](struct.Rotation.html#method.new) function:
///
/// ### MB 100 Rotation, with 9 backups
/// ```rust
/// # fn docs() {
/// use tracing_appender::rotating::Rotation;
/// let rotation = tracing_appender::rotating::Rotation::mb_100();
/// # }
/// ```
///
/// ### No Rotation
/// ```rust
/// # fn docs() {
/// use tracing_appender::rotating::Rotation;
/// let rotation = tracing_appender::rotating::Rotation::never();
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Rotation {
    max_bytes: usize,
    max_backups: usize,
    max_backups_digit: usize,
}

impl Rotation {
    /// Provides a 100 MB rotation with 9 max_backups
    pub fn mb_100() -> Self {
        Rotation::new(100_000_000, 9)
    }

    /// Provides a rotation that never rotates.
    pub fn never() -> Self {
        Rotation::new(0, 0)
    }
}
impl Rotation {
    /// Creates a new `Rotation`.
    ///
    /// A `Rotation` will have a fixed number whose size is
    /// defined by [`Rotation`](struct.Rotation.html). The `max_bytes` argument
    /// define the number of bytes per log file. The `max_backups` argument define the number
    /// of old log files allowed.
    ///
    /// Alternatively, a `Rotation` can be constructed using one of the following helpers:
    ///
    /// - [`Rotation::mb_100()`][mb_100]
    /// - [`Rotation::never()`][never]
    ///
    /// [mb_100]: struct.Rotation.html#method.mb_100
    /// [never]: struct.Rotation.html#method.never
    ///
    /// # Examples
    /// ```rust
    /// # fn docs() {
    /// use tracing_appender::rotating::Rotation;
    /// let rotation = tracing_appender::rotating::Rotation::new(100_000_000,99);
    /// # }
    /// ```
    pub fn new(max_bytes: usize, max_backups: usize) -> Self {
        Self {
            max_backups_digit: max_backups.to_string().len(),
            max_bytes,
            max_backups,
        }
    }
    pub(crate) fn should_rollover(&self, size: usize) -> bool {
        size > self.max_bytes && self.max_bytes != 0
    }
    pub(crate) fn is_create_backup(&self, last_backup: usize) -> bool {
        last_backup < self.max_backups
    }
    pub(crate) fn join_backup(&self, filename: &str, backup_index: usize) -> String {
        match backup_index {
            0 => filename.to_string(),
            _ => format!("{}.{}", filename, self.backup_index_to_str(backup_index)),
        }
    }
    fn backup_index_to_str(&self, backup_index: usize) -> String {
        let backup_index_str = backup_index.to_string();
        let backup_index_len = backup_index_str.len();
        if backup_index_len < self.max_backups_digit {
            std::iter::repeat("0")
                .take(self.max_backups_digit - backup_index_len)
                .collect::<String>()
                + &backup_index_str
        } else {
            backup_index_str
        }
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

    fn write_to_log(appender: &mut RotatingFileAppender, msg: &str) {
        appender
            .write_all(msg.as_bytes())
            .expect("Failed to write to appender");
        appender.flush().expect("Failed to flush!");
    }

    fn test_appender(rotation: Rotation, directory: TempDir, file_prefix: &str) {
        let mut appender = RotatingFileAppender::new(rotation, directory.path(), file_prefix);

        let expected_value = "Hello";
        write_to_log(&mut appender, expected_value);
        assert!(find_str_in_log(directory.path(), expected_value));

        directory
            .close()
            .expect("Failed to explicitly close TempDir. TempDir should delete once out of scope.")
    }
    #[test]
    fn rotating_write() {
        let r = Rotation::new(10, 2);
        let directory = TempDir::new("rotating").expect("Failed to create tempdir");
        let file_prefix = "rotating.log";
        let mut appender = RotatingFileAppender::new(r, directory.path(), file_prefix);
        write_to_log(&mut appender, "1111");
        write_to_log(&mut appender, "2222");
        write_to_log(&mut appender, "3333");
        write_to_log(&mut appender, "4444");
        write_to_log(&mut appender, "5555");
        write_to_log(&mut appender, "6666");
        write_to_log(&mut appender, "7777");
        write_to_log(&mut appender, "8888");

        assert_eq!(
            fs::read_to_string(directory.path().join("rotating.log")).unwrap(),
            "77778888"
        );
        assert_eq!(
            fs::read_to_string(directory.path().join("rotating.log.1")).unwrap(),
            "55556666"
        );

        assert_eq!(
            fs::read_to_string(directory.path().join("rotating.log.2")).unwrap(),
            "33334444"
        );
        assert_eq!(directory.path().join("rotating.log.3").exists(), false);
    }
    #[test]
    fn rotating_double_write() {
        let directory = TempDir::new("rotating").expect("Failed to create tempdir");
        let file_prefix = "rotating.log";
        {
            let r = Rotation::new(10, 2);
            let mut appender = RotatingFileAppender::new(r, directory.path(), file_prefix);
            write_to_log(&mut appender, "1111");
            write_to_log(&mut appender, "2222");
            write_to_log(&mut appender, "3333");
        }
        let r = Rotation::new(10, 2);
        let mut appender = RotatingFileAppender::new(r, directory.path(), file_prefix);
        write_to_log(&mut appender, "4444");
        write_to_log(&mut appender, "5555");
        write_to_log(&mut appender, "6666");
        write_to_log(&mut appender, "7777");
        write_to_log(&mut appender, "8888");

        assert_eq!(
            fs::read_to_string(directory.path().join("rotating.log")).unwrap(),
            "77778888"
        );
        assert_eq!(
            fs::read_to_string(directory.path().join("rotating.log.1")).unwrap(),
            "55556666"
        );

        assert_eq!(
            fs::read_to_string(directory.path().join("rotating.log.2")).unwrap(),
            "33334444"
        );
        assert_eq!(directory.path().join("rotating.log.3").exists(), false);
    }
    #[test]
    fn write_mb_100_log() {
        test_appender(
            Rotation::mb_100(),
            TempDir::new("mb_100").expect("Failed to create tempdir"),
            "mb_100.log",
        );
    }

    #[test]
    fn write_never_log() {
        test_appender(
            Rotation::never(),
            TempDir::new("never").expect("Failed to create tempdir"),
            "never.log",
        );
    }

    #[test]
    fn test_should_rollover() {
        let r = Rotation::new(200, 0);
        assert_eq!(r.should_rollover(999), true);
        assert_eq!(r.should_rollover(0), false);
        assert_eq!(r.should_rollover(200), false);
        assert_eq!(r.should_rollover(201), true);

        let r = Rotation::new(0, 0);
        assert_eq!(r.should_rollover(99999), false);
    }

    #[test]
    fn test_is_create_backup() {
        let r = Rotation::new(0, 999);
        assert_eq!(r.is_create_backup(999), false);
        assert_eq!(r.is_create_backup(0), true);
        assert_eq!(r.is_create_backup(100), true);

        let r = Rotation::new(0, 0);
        assert_eq!(r.is_create_backup(999), false);
    }
    #[test]
    fn test_join_backup() {
        let r = Rotation::new(0, 999);
        let joined_date = r.join_backup("Hello.log", 0);
        assert_eq!(joined_date, "Hello.log");
        let joined_date = r.join_backup("Hello.log", 1);
        assert_eq!(joined_date, "Hello.log.001");
        let joined_date = r.join_backup("Hello.log", 22);
        assert_eq!(joined_date, "Hello.log.022");
        let joined_date = r.join_backup("Hello.log", 333);
        assert_eq!(joined_date, "Hello.log.333");
        let joined_date = r.join_backup("Hello.log", 4444);
        assert_eq!(joined_date, "Hello.log.4444");
        let r = Rotation::new(0, 0);
        let joined_date = r.join_backup("Hello.log", 22);
        assert_eq!(joined_date, "Hello.log.22");
    }
}
