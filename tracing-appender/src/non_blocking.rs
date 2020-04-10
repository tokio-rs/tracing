use crate::worker::Worker;
use crossbeam_channel::{bounded, Sender};
use std::io;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;
use tracing_subscriber::fmt::MakeWriter;

pub const DEFAULT_BUFFERED_LINES_LIMIT: usize = 128_000;

#[derive(Clone, Debug)]
pub struct NonBlocking {
    _worker_guard: Arc<JoinHandle<()>>,
    error_counter: Arc<AtomicU64>,
    channel: Sender<Vec<u8>>,
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
            channel: sender,
            error_counter: error_counter.clone(),
            _worker_guard: Arc::new(worker.worker_thread()),
        }
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
            buffered_lines_limit: DEFAULT_BUFFERED_LINES_LIMIT,
        }
    }
}

impl std::io::Write for NonBlocking {
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

impl MakeWriter for NonBlocking {
    type Writer = NonBlocking;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}
