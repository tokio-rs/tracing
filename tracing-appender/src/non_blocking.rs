use crate::worker::Worker;
use crossbeam_channel::{bounded, Sender};
use std::io;
use std::io::Write;
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
    is_lossy: bool,
}

impl NonBlocking {
    pub fn new<T: Write + Send + Sync + 'static>(writer: T) -> NonBlocking {
        NonBlockingBuilder::default().build(writer)
    }

    fn create<T: Write + Send + Sync + 'static>(
        sender: Sender<Vec<u8>>,
        error_counter: Arc<AtomicU64>,
        worker: Worker<T>,
        is_lossy: bool,
    ) -> NonBlocking {
        Self {
            channel: sender,
            error_counter: error_counter.clone(),
            _worker_guard: Arc::new(worker.worker_thread()),
            is_lossy,
        }
    }

    pub fn error_counter(&self) -> Arc<AtomicU64> {
        self.error_counter.clone()
    }
}

#[derive(Debug)]
pub struct NonBlockingBuilder {
    buffered_lines_limit: usize,
    is_lossy: bool,
}

impl NonBlockingBuilder {
    pub fn buffered_lines_limit(mut self, buffered_lines_limit: usize) -> NonBlockingBuilder {
        self.buffered_lines_limit = buffered_lines_limit;
        self
    }

    pub fn lossy(mut self, is_lossy: bool) -> NonBlockingBuilder {
        self.is_lossy = is_lossy;
        self
    }

    pub fn build<'a, T: Write + Send + Sync + 'static>(self, writer: T) -> NonBlocking {
        let (sender, receiver) = bounded(self.buffered_lines_limit);
        let worker = Worker::new(receiver, writer);

        NonBlocking::create(sender, Arc::new(AtomicU64::new(0)), worker, self.is_lossy)
    }
}

impl Default for NonBlockingBuilder {
    fn default() -> Self {
        NonBlockingBuilder {
            buffered_lines_limit: DEFAULT_BUFFERED_LINES_LIMIT,
            is_lossy: true,
        }
    }
}

impl std::io::Write for NonBlocking {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let buf_size = buf.len();
        if self.is_lossy {
            if self.channel.try_send(buf.to_vec()).is_err() {
                self.error_counter.fetch_add(1, Ordering::Relaxed);
            }
        } else {
            self.channel.send(buf.to_vec());
        }
        Ok(buf_size)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.write(buf).map(|_| ())
    }
}

impl MakeWriter for NonBlocking {
    type Writer = NonBlocking;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}
