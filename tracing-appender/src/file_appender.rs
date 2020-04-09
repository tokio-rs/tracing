use crate::inner::BufWriterFactory;
use crate::worker::Worker;
use crate::Rotation;
use chrono::Utc;
use crossbeam_channel::{bounded, Sender};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::{io, thread};
use tracing_subscriber::fmt::MakeWriter;

#[allow(dead_code)]
pub struct FileAppender {
    log_writer: FileWriter,
    worker_thread: thread::JoinHandle<()>,
    error_counter: Arc<AtomicU64>,
}

impl FileAppender {
    pub fn new(log_directory: &str, log_filename_prefix: &str) -> Self {
        FileAppenderBuilder::default()
            .build(log_directory, log_filename_prefix)
            .expect("Failed to create FileAppender")
    }

    fn create_appender(
        sender: Sender<Vec<u8>>,
        worker: Worker<BufWriterFactory>,
        error_counter: Arc<AtomicU64>,
    ) -> Self {
        Self {
            log_writer: FileWriter {
                channel: sender,
                error_counter: error_counter.clone(),
            },
            worker_thread: worker.worker_thread(),
            error_counter,
        }
    }

    pub fn builder() -> FileAppenderBuilder {
        FileAppenderBuilder::default()
    }

    pub fn writer(&self) -> FileWriter {
        self.log_writer.clone()
    }

    pub fn error_counter(&self) -> Arc<AtomicU64> {
        self.error_counter.clone()
    }
}

pub struct FileAppenderBuilder {
    buffered_lines_limit: usize,
    rotation: Rotation,
}

impl FileAppenderBuilder {
    pub fn buffered_lines_limit(mut self, buffered_lines_limit: usize) -> FileAppenderBuilder {
        self.buffered_lines_limit = buffered_lines_limit;
        self
    }

    pub fn build(
        self,
        log_directory: impl AsRef<Path>,
        log_filename_prefix: impl AsRef<Path>,
    ) -> io::Result<FileAppender> {
        let (sender, receiver) = bounded(self.buffered_lines_limit);

        let worker = Worker::new(
            receiver,
            log_directory.as_ref(),
            log_filename_prefix.as_ref(),
            self.rotation,
            BufWriterFactory {},
            Utc::now(),
        );
        Ok(FileAppender::create_appender(
            sender,
            worker?,
            Arc::new(AtomicU64::new(0)),
        ))
    }
}

impl Default for FileAppenderBuilder {
    fn default() -> Self {
        FileAppenderBuilder {
            buffered_lines_limit: 100_000,
            rotation: Rotation::HOURLY,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FileWriter {
    channel: Sender<Vec<u8>>,
    error_counter: Arc<AtomicU64>,
}

impl std::io::Write for FileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let buf_size = buf.len();
        if self.channel.try_send(buf.to_vec()).is_err() {
            self.error_counter.fetch_add(1, Ordering::Relaxed);
        }
        Ok(buf_size)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl MakeWriter for FileWriter {
    type Writer = FileWriter;

    fn make_writer(&self) -> Self::Writer {
        FileWriter {
            channel: self.channel.clone(),
            error_counter: self.error_counter.clone(),
        }
    }
}
