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
use {
    crate::rolling::compression::CompressionConfig, crate::rolling::compression::CompressionOption,
};

use crate::writer::WriterChannel;

/// Struct for keeping temporary values of `RollingFileAppender`.
///
/// Note that `log_directory` and `log_filename_prefix` are obligatory parameters and should
/// be passed into the constructor of `RollingFileAppenderBuilder`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RollingFileAppenderBuilder {
    log_directory: String,
    log_filename_prefix: String,
    rotation: Option<Rotation>,
    #[cfg(feature = "compression_gzip")]
    compression: Option<CompressionConfig>,
}

impl RollingFileAppenderBuilder {
    /// Creates a new `RollingFileAppnderBuilder`
    ///
    /// It was introduced to open up the possibility to use `compression` without
    /// breaking the current interface.
    ///
    /// Note that `compression` module is enabled by using an optional feature flag
    /// `compression_gzip` (for gzip algorithm)
    ///
    /// # Examples
    /// ```rust
    /// use tracing_appender::builder::RollingFileAppenderBuilder;
    /// use tracing_appender::compression::CompressionOption;
    /// use tracing_appender::rolling::Rotation;
    /// let builder = RollingFileAppenderBuilder::new("/var/tmp", "my-app")
    ///     .rotation(Rotation::DAILY)
    ///     .compression(CompressionOption::GzipFast);
    /// ```
    pub fn new(log_directory: impl AsRef<Path>, log_filename_prefix: impl AsRef<Path>) -> Self {
        let log_directory = log_directory
            .as_ref()
            .to_str()
            .expect("Cannot convert log_directory Path to str")
            .to_string();
        let log_filename_prefix = log_filename_prefix
            .as_ref()
            .to_str()
            .expect("Cannot convert log_filename_prefix Path to str")
            .to_string();
        RollingFileAppenderBuilder {
            log_directory,
            log_filename_prefix,
            rotation: None,
            #[cfg(feature = "compression_gzip")]
            #[cfg_attr(docsrs, doc(cfg(feature = "compression_gzip")))]
            compression: None,
        }
    }

    /// Sets Rotation
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

    /// Builds an instance of `RollingFileAppender` using previously defined attributes.
    pub fn build(self) -> RollingFileAppender {
        let now = OffsetDateTime::now_utc();
        let rotation = self.rotation.clone().unwrap_or(Rotation::NEVER).clone();
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
                rotation: rotation.clone(),
                #[cfg(feature = "compression_gzip")]
                compression: self.compression,
            },
            writer,
        }
    }

    fn create_file_writer(&self, filename: &str) -> RwLock<WriterChannel> {
        let a = RwLock::new(WriterChannel::File(
            create_writer_file(self.log_directory.as_str(), &filename)
                .expect("failed to create appender"),
        ));
        a
    }
}
