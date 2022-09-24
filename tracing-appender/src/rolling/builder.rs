use super::{RollingFileAppender, Rotation};
use std::{io, path::Path};
use thiserror::Error;

/// A [builder] for configuring [`RollingFileAppender`]s.
///
/// [builder]: https://rust-unofficial.github.io/patterns/patterns/creational/builder.html
#[derive(Debug)]
pub struct Builder {
    pub(super) rotation: Rotation,
    pub(super) prefix: Option<String>,
    pub(super) suffix: Option<String>,
}

/// Errors returned by [`Builder::build`].
#[derive(Error, Debug)]
#[error("{context}: {source}")]
pub struct InitError {
    context: &'static str,
    #[source]
    source: io::Error,
}

impl InitError {
    pub(crate) fn ctx(context: &'static str) -> impl FnOnce(io::Error) -> Self {
        move |source| Self { context, source }
    }
}

impl Builder {
    /// Returns a new `Builder` for configuring a [`RollingFileAppender`], with
    /// the default parameters.
    ///
    /// # Default Values
    ///
    /// The default values for the builder are:
    ///
    /// | Parameter | Default Value | Notes |
    /// | :-------- | :------------ | :---- |
    /// | [`rotation`] | [`Rotation::NEVER`] | By default, log files will never be rotated. |
    /// | [`filename_prefix`] | `""` | By default, log file names will not have a prefix. |
    ///
    /// [`rotation`]: Self::rotation
    /// [`filename_prefix`]: Self::filename_prefix
    #[must_use]
    pub const fn new() -> Self {
        Self {
            rotation: Rotation::NEVER,
            prefix: None,
            suffix: None,
        }
    }

    /// Sets the [rotation strategy] for log files.
    ///
    /// By default, this is [`Rotation::NEVER`].
    ///
    /// # Examples
    ///
    /// ```
    /// # fn docs() {
    /// use tracing_appender::rolling::{Rotation, RollingFileAppender};
    ///
    /// let appender = RollingFileAppender::builder()
    ///     .rotation(Rotation::HOURLY) // rotate log files once every hour
    ///     // ...
    ///     .build("/var/log")
    ///     .expect("failed to initialize rolling file appender");
    ///
    /// # drop(appender)
    /// # }
    /// ```
    ///
    /// [rotation strategy]: Rotation
    #[must_use]
    pub fn rotation(self, rotation: Rotation) -> Self {
        Self { rotation, ..self }
    }

    /// Sets the prefix for log filenames. The prefix is output before the
    /// timestamp in the file name, and if it is non-empty, it is followed by a
    /// dot (`.`).
    ///
    /// By default, log files do not have a prefix.
    ///
    /// # Examples
    ///
    /// Setting a prefix:
    ///
    /// ```
    /// use tracing_appender::rolling::RollingFileAppender;
    ///
    /// # fn docs() {
    /// let appender = RollingFileAppender::builder()
    ///     .filename_prefix("myapp.log") // log files will have names like "myapp.log.2019-01-01"
    ///     // ...
    ///     .build("/var/log")
    ///     .expect("failed to initialize rolling file appender");
    /// # drop(appender)
    /// # }
    /// ```
    ///
    /// No prefix:
    ///
    /// ```
    /// use tracing_appender::rolling::RollingFileAppender;
    ///
    /// # fn docs() {
    /// let appender = RollingFileAppender::builder()
    ///     .filename_prefix("") // log files will have names like "2019-01-01"
    ///     // ...
    ///     .build("/var/log")
    ///     .expect("failed to initialize rolling file appender");
    /// # drop(appender)
    /// # }
    /// ```
    ///
    /// [rotation strategy]: Rotation
    #[must_use]
    pub fn filename_prefix(self, prefix: impl Into<String>) -> Self {
        let prefix = prefix.into();
        // If the configured prefix is the empty string, then don't include a
        // separator character.
        let prefix = if prefix.is_empty() {
            None
        } else {
            Some(prefix)
        };
        Self { prefix, ..self }
    }

    /// Sets the suffix for log filenames. The suffix is output after the
    /// timestamp in the file name, and if it is non-empty, it is preceded by a
    /// dot (`.`).
    ///
    /// By default, log files do not have a suffix.
    ///
    /// # Examples
    ///
    /// Setting a suffix:
    ///
    /// ```
    /// use tracing_appender::rolling::RollingFileAppender;
    ///
    /// # fn docs() {
    /// let appender = RollingFileAppender::builder()
    ///     .filename_suffix("myapp.log") // log files will have names like "2019-01-01.myapp.log"
    ///     // ...
    ///     .build("/var/log")
    ///     .expect("failed to initialize rolling file appender");
    /// # drop(appender)
    /// # }
    /// ```
    ///
    /// No suffix:
    ///
    /// ```
    /// use tracing_appender::rolling::RollingFileAppender;
    ///
    /// # fn docs() {
    /// let appender = RollingFileAppender::builder()
    ///     .filename_suffix("") // log files will have names like "2019-01-01"
    ///     // ...
    ///     .build("/var/log")
    ///     .expect("failed to initialize rolling file appender");
    /// # drop(appender)
    /// # }
    /// ```
    ///
    /// [rotation strategy]: Rotation
    #[must_use]
    pub fn filename_suffix(self, suffix: impl Into<String>) -> Self {
        let suffix = suffix.into();
        // If the configured suffix is the empty string, then don't include a
        // separator character.
        let suffix = if suffix.is_empty() {
            None
        } else {
            Some(suffix)
        };
        Self { suffix, ..self }
    }

    /// Builds a new [`RollingFileAppender`] with the configured parameters,
    /// emitting log files to the provided directory.
    ///
    /// Unlike [`RollingFileAppender::new`], this returns a `Result` rather than
    /// panicking when the appender cannot be initialized.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_appender::rolling::{Rotation, RollingFileAppender};
    ///
    /// # fn docs() {
    /// let appender = RollingFileAppender::builder()
    ///     .rotation(Rotation::DAILY) // rotate log files once per day
    ///     .filename_prefix("myapp.log") // log files will have names like "myapp.log.2019-01-01"
    ///     .build("/var/log/myapp") // write log files to the '/var/log/myapp' directory
    ///     .expect("failed to initialize rolling file appender");
    /// # drop(appender);
    /// # }
    /// ```
    ///
    /// This is equivalent to
    /// ```
    /// # fn docs() {
    /// let appender = tracing_appender::rolling::daily("myapp.log", "/var/log/myapp");
    /// # drop(appender);
    /// # }
    /// ```
    pub fn build(&self, directory: impl AsRef<Path>) -> Result<RollingFileAppender, InitError> {
        RollingFileAppender::from_builder(self, directory)
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}
