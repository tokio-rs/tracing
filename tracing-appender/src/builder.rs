use crate::rolling::{create_writer_file, Inner, RollingFileAppender, Rotation};
use crate::sync::RwLock;
use std::sync::atomic::AtomicUsize;
use time::OffsetDateTime;

#[cfg(feature = "compression")]
use crate::compression::CompressionConfig;

use crate::writer::WriterChannel;

pub struct RollingFileAppenderBuilder {
    log_directory: Option<String>,
    log_filename_prefix: Option<String>,
    rotation: Option<Rotation>,
    next_date: Option<AtomicUsize>,
    #[cfg(feature = "compression")]
    compression: Option<CompressionConfig>,
}

impl RollingFileAppenderBuilder {
    pub fn new() -> Self {
        RollingFileAppenderBuilder {
            log_directory: None,
            log_filename_prefix: None,
            rotation: None,
            next_date: None,
            #[cfg(feature = "compression")]
            compression: None,
        }
    }

    pub fn log_directory(mut self, log_directory: String) -> Self {
        self.log_directory = Some(log_directory);
        self
    }

    pub fn log_filename_prefix(mut self, log_filename_prefix: String) -> Self {
        self.log_filename_prefix = Some(log_filename_prefix);
        self
    }

    pub fn rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = Some(rotation);
        self
    }

    pub fn next_date(mut self, next_date: AtomicUsize) -> Self {
        self.next_date = Some(next_date);
        self
    }

    #[cfg(feature = "compression")]
    pub fn compression(mut self, compression: CompressionConfig) -> Self {
        self.compression = Some(compression);
        self
    }

    pub fn build(self) -> RollingFileAppender {
        let now = OffsetDateTime::now_utc();
        let log_directory = self
            .log_directory
            .expect("log_directory is required to build RollingFileAppender");
        let log_filename_prefix = self
            .log_filename_prefix
            .expect("log_filename_prefix is required to build RollingFileAppender");
        let rotation = self
            .rotation
            .expect("rotation is required to build RollingFileAppender");

        let filename = rotation.join_date(log_filename_prefix.as_str(), &now, false);
        let next_date = rotation.next_date(&now);
        let writer = RwLock::new(WriterChannel::File(
            create_writer_file(log_directory.as_str(), &filename)
                .expect("failed to create appender"),
        ));

        let next_date = AtomicUsize::new(
            next_date
                .map(|date| date.unix_timestamp() as usize)
                .unwrap_or(0),
        );

        RollingFileAppender {
            state: Inner {
                log_directory,
                log_filename_prefix,
                next_date,
                rotation,
                #[cfg(feature = "compression")]
                compression: self.compression,
            },
            writer,
        }
    }
}
