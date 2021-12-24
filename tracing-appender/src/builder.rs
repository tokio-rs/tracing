use std::path::Path;
use crate::rolling::{create_writer_file, Inner, RollingFileAppender, Rotation};
use crate::sync::RwLock;
use std::sync::atomic::AtomicUsize;
use time::OffsetDateTime;

#[cfg(feature = "compression")]
use crate::compression::CompressionConfig;

use crate::writer::WriterChannel;


#[derive(Debug)]
pub struct RollingFileAppenderBuilder {
    log_directory: String,
    log_filename_prefix: String,
    rotation: Option<Rotation>,
    #[cfg(feature = "compression")]
    compression: Option<CompressionConfig>,
}

impl RollingFileAppenderBuilder {
    /// Creates an instance of RollingFileAppnderBuilder
    pub fn new(log_directory: impl AsRef<Path>,
               log_filename_prefix: impl AsRef<Path>) -> Self {
        let log_directory = log_directory.as_ref().to_str().expect("Cannot convert log_directory Path to str").to_string();
        let log_filename_prefix = log_filename_prefix.as_ref().to_str().expect("Cannot convert log_filename_prefix Path to str").to_string();
        RollingFileAppenderBuilder {
            log_directory,
            log_filename_prefix,
            rotation: None,
            #[cfg(feature = "compression")]
            #[cfg_attr(docsrs, doc(cfg(feature = "compression")))]
            compression: None,
        }
    }

    /// Sets Rotation
    pub fn rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = Some(rotation);
        self
    }

    #[cfg(feature = "compression")]
    #[cfg_attr(docsrs, doc(cfg(feature = "compression")))]
    pub fn compression(mut self, compression: CompressionConfig) -> Self {
        self.compression = Some(compression);
        self
    }

    pub fn build(self) -> RollingFileAppender {
        let now = OffsetDateTime::now_utc();
        let rotation = self.rotation.unwrap_or(Rotation::NEVER);
        let filename = rotation.join_date(self.log_filename_prefix.as_str(), &now, false);
        let next_date = rotation.next_date(&now);

        let writer = RwLock::new(WriterChannel::File(
            create_writer_file(self.log_directory.as_str(), &filename)
                .expect("failed to create appender"),
        ));

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
                rotation: rotation,
                #[cfg(feature = "compression")]
                compression: self.compression,
            },
            writer,
        }
    }
}
