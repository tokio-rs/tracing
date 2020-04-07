use crate::inner::BufWriterFactory;
use crate::worker::Worker;
use crate::Rotation;
use chrono::Utc;
use crossbeam_channel::{bounded, Sender};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{io, thread};
use tracing_subscriber::fmt::MakeWriter;

#[allow(dead_code)]
pub struct FileAppender {
    log_writer: FileWriter,
    worker_thread: thread::JoinHandle<()>,
}

impl FileAppender {
    fn new(
        sender: Sender<Vec<u8>>,
        worker: Worker<BufWriterFactory>,
        error_counter: &'static AtomicU64,
    ) -> Self {
        Self {
            log_writer: FileWriter {
                channel: sender,
                error_counter,
            },
            worker_thread: worker.worker_thread(),
        }
    }

    pub fn builder() -> FileAppenderBuilder {
        FileAppenderBuilder::default()
    }

    pub fn get_writer(self) -> FileWriter {
        self.log_writer
    }
}

pub struct FileAppenderBuilder {
    buffered_lines_limit: usize,
    rotation: Rotation,
}

#[allow(dead_code)]
impl FileAppenderBuilder {
    pub fn buffered_lines_limit(mut self, buffered_lines_limit: usize) -> FileAppenderBuilder {
        self.buffered_lines_limit = buffered_lines_limit;
        self
    }

    pub fn build(
        self,
        log_directory: &str,
        log_filename_prefix: &str,
        error_counter: &'static AtomicU64,
    ) -> io::Result<FileAppender> {
        let (sender, receiver) = bounded(self.buffered_lines_limit);

        let worker = Worker::new(
            receiver,
            log_directory,
            log_filename_prefix,
            self.rotation,
            BufWriterFactory {},
            Utc::now(),
        );
        Ok(FileAppender::new(sender, worker?, error_counter))
    }
}

impl Default for FileAppenderBuilder {
    fn default() -> Self {
        FileAppenderBuilder {
            buffered_lines_limit: 100_000,
            rotation: Rotation::Hourly,
        }
    }
}

pub struct FileWriter {
    channel: Sender<Vec<u8>>,
    error_counter: &'static AtomicU64,
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
        unimplemented!()
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
