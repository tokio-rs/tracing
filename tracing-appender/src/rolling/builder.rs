//! Builder struct for `RollingFileAppender`
//!
//! Gives access to setting additional options which are not avaible using standard interface.
//! Currently it is the only way to enable compression of logs.
use crate::rolling::{create_writer_file, Inner, RollingFileAppender, Rotation};
use crate::sync::RwLock;
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use time::OffsetDateTime;

#[cfg(feature = "compression_gzip")]
use crate::rolling::compression::{CompressionConfig, CompressionOption};

use crate::writer::WriterChannel;

/// A builder for configuring new [`RollingFileAppender`]s.
///
/// Note that `log_directory` and `log_filename_prefix` are obligatory parameters and should
/// be passed into the constructor of `RollingFileAppenderBuilder`.
#[derive(Debug, Clone)]
pub struct Builder {
    log_directory: String,
    log_filename_prefix: String,
    rotation: Option<Rotation>,
    #[cfg(feature = "compression_gzip")]
    compression: Option<CompressionConfig>,
}

impl Builder {
    /// Returns a new builder for configuring a [`RollingFileAppender`], with
    /// the provided log directory and filename prefix.
    ///
    /// Calling [`build`] on the returned builder will construct a new
    /// [`RollingFileAppender`] that writes to files in the provided directory.
    /// By default, log files will never be rotated and compression will not be
    /// enabled. A rotation policy can be added to the builder using the
    /// [`rotation`] method. When the "compression" feature flag is enabled,
    /// compression can be configured using the [`compression`] method.
    ///
    /// # Panics
    ///
    /// This function panics if the provided log directory or log file prefix
    /// are not valid UTF-8.
    ///
    /// # Examples
    ///
    /// Building a `RollingFileAppender` with the default configuration:
    ///
    /// ```rust
    /// use tracing_appender::rolling::Builder;
    /// let appender = Builder::new("/var/tmp", "my-app")
    ///     .build();
    /// ```
    ///
    /// Enabling compression (needs a feature enabled `compression_gzip`):
    ///
    /// ```rust
    /// #[cfg(feature = "compression_gzip")]
    /// use tracing_appender::{
    ///     rolling::{Builder, Rotation},
    ///     rolling::compression::CompressionOption,
    /// };
    /// #[cfg(feature = "compression_gzip")]
    /// let appender = Builder::new("/var/tmp", "my-app")
    ///     .rotation(Rotation::DAILY)
    ///     .compression(CompressionOption::GzipFast)
    ///     .build();
    /// ```
    pub fn new(log_directory: impl AsRef<Path>, log_filename_prefix: impl AsRef<Path>) -> Self {
        let log_directory = log_directory
            .as_ref()
            .to_str()
            .expect("`log_directory` must not contain invalid UTF-8 characters")
            .to_string();
        let log_filename_prefix = log_filename_prefix
            .as_ref()
            .to_str()
            .expect("`log_directory` must not contain invalid UTF-8 characters")
            .to_string();
        Builder {
            log_directory,
            log_filename_prefix,
            rotation: None,
            #[cfg(feature = "compression_gzip")]
            compression: None,
        }
    }

    /// Configures when log files will be rotated.
    ///
    /// By default, no rotation will occur.
    pub fn rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = Some(rotation);
        self
    }

    /// Sets compression level
    #[cfg(feature = "compression_gzip")]
    #[cfg_attr(docsrs, doc(cfg(feature = "compression_gzip")))]
    pub fn compression(mut self, compression: CompressionOption) -> Self {
        self.compression = Some(compression.into());
        self
    }

    pub(crate) fn get_extension(&self) -> Option<String> {
        #[cfg(feature = "compression_gzip")]
        if let Some(compression) = self.compression.clone() {
            compression.extension().map(|v| v.to_string())
        } else {
            None
        }

        #[cfg(not(feature = "compression_gzip"))]
        None
    }

    /// Returns a new [`RollingFileAppender`] with the configuration defined by this builder.
    pub fn build(self) -> RollingFileAppender {
        let now = OffsetDateTime::now_utc();
        let rotation = self.rotation.clone().unwrap_or(Rotation::NEVER);
        let extension = self.get_extension();
        let filename = rotation.join_date(self.log_filename_prefix.as_str(), &now, extension);
        let next_date = rotation.next_date(&now);

        #[cfg(not(feature = "compression_gzip"))]
        let writer = self.create_file_writer(filename.as_str());

        #[cfg(feature = "compression_gzip")]
        let writer = if let Some(compression) = self.compression.clone() {
            RwLock::new(
                WriterChannel::new_with_compression(
                    self.log_directory.as_str(),
                    &filename,
                    compression,
                )
                .unwrap(),
            )
        } else {
            self.create_file_writer(filename.as_str())
        };

        let next_date = AtomicUsize::new(
            next_date
                .map(|date| date.unix_timestamp() as usize)
                .unwrap_or(0),
        );

        RollingFileAppender {
            state: Inner {
                log_directory: self.log_directory,
                log_filename_prefix: self.log_filename_prefix,
                next_date,
                rotation,
                #[cfg(feature = "compression_gzip")]
                compression: self.compression,
            },
            writer,
        }
    }

    fn create_file_writer(&self, filename: &str) -> RwLock<WriterChannel> {
        let a = RwLock::new(WriterChannel::File(
            create_writer_file(self.log_directory.as_str(), filename)
                .expect("failed to create appender"),
        ));
        a
    }
}
