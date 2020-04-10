use crate::worker::Worker;
use crossbeam_channel::{bounded, Sender};
use std::io;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;
use tracing_subscriber::fmt::MakeWriter;


pub struct NonBlocking {
    writer: NonBlockingWriter,
    _worker_guard: JoinHandle<()>,
    error_counter: Arc<AtomicU64>,
}

#[derive(Clone, Debug)]
pub struct NonBlockingWriter {
    channel: Sender<Vec<u8>>,
    error_counter: Arc<AtomicU64>,
}

impl NonBlocking {
    pub fn new<'a, T: MakeWriter + Send + Sync + 'static>(make_writer: T) -> NonBlocking {
        NonBlockingBuilder::default().build(make_writer)
    }

    fn create<'a, T: MakeWriter + Send + Sync + 'static>(
        sender: Sender<Vec<u8>>,
        error_counter: Arc<AtomicU64>,
        worker: Worker<T>,
    ) -> NonBlocking {
        Self {
            writer: NonBlockingWriter {
                channel: sender,
                error_counter: error_counter.clone(),
            },
            _worker_guard: worker.worker_thread(),
            error_counter
        }
    }

    pub fn writer(&self) -> NonBlockingWriter {
        self.writer.clone()
    }

    pub fn error_counter(&self) -> Arc<AtomicU64> {
        self.error_counter.clone()
    }
}

#[derive(Debug)]
pub struct NonBlockingBuilder {
    buffered_lines_limit: usize,
}

impl NonBlockingBuilder {
    pub fn buffered_lines_limit(mut self, buffered_lines_limit: usize) -> NonBlockingBuilder {
        self.buffered_lines_limit = buffered_lines_limit;
        self
    }

    pub fn build<'a, T: MakeWriter + Send + Sync + 'static>(self, make_writer: T) -> NonBlocking {
        let (sender, receiver) = bounded(self.buffered_lines_limit);
        let worker = Worker::new(receiver, make_writer);

        NonBlocking::create(sender, Arc::new(AtomicU64::new(0)), worker)
    }
}

impl Default for NonBlockingBuilder {
    fn default() -> Self {
        NonBlockingBuilder {
            buffered_lines_limit: 100_000,
        }
    }
}

impl std::io::Write for NonBlockingWriter {
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

impl MakeWriter for NonBlockingWriter {
    type Writer = NonBlockingWriter;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}
