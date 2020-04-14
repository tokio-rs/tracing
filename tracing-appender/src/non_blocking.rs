use crate::worker::Worker;
use crossbeam_channel::{bounded, Sender};
use std::io;
use std::io::Write;
use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;
use std::thread::JoinHandle;
use tracing_subscriber::fmt::MakeWriter;

pub const DEFAULT_BUFFERED_LINES_LIMIT: usize = 128_000;

#[derive(Debug)]
pub struct WorkerGuard {
    guard: Option<JoinHandle<()>>,
    shutdown_signal: Arc<AtomicBool>,
}

#[derive(Clone, Debug)]
pub struct NonBlocking {
    error_counter: Arc<AtomicU64>,
    channel: Sender<Vec<u8>>,
    is_lossy: bool,
}

impl NonBlocking {
    pub fn new<T: Write + Send + Sync + 'static>(writer: T) -> (NonBlocking, WorkerGuard) {
        NonBlockingBuilder::default().finish(writer)
    }

    fn create<T: Write + Send + Sync + 'static>(
        writer: T,
        buffered_lines_limit: usize,
        is_lossy: bool,
    ) -> (NonBlocking, WorkerGuard) {
        let (sender, receiver) = bounded(buffered_lines_limit);
        let shutdown_signal = Arc::new(AtomicBool::new(false));

        let worker = Worker::new(receiver, writer, shutdown_signal.clone());
        let worker_guard = WorkerGuard::new(worker.worker_thread(), shutdown_signal);

        (
            Self {
                channel: sender,
                error_counter: Arc::new(AtomicU64::new(0)),
                is_lossy,
            },
            worker_guard,
        )
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

    pub fn finish<T: Write + Send + Sync + 'static>(self, writer: T) -> (NonBlocking, WorkerGuard) {
        NonBlocking::create(writer, self.buffered_lines_limit, self.is_lossy)
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
            return match self.channel.send(buf.to_vec()) {
                Ok(_) => Ok(buf_size),
                Err(_) => Err(io::Error::from(io::ErrorKind::Other)),
            };
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

impl WorkerGuard {
    fn new(handle: JoinHandle<()>, shutdown_signal: Arc<AtomicBool>) -> Self {
        WorkerGuard {
            guard: Some(handle),
            shutdown_signal,
        }
    }

    fn stop(&mut self) -> std::thread::Result<()> {
        match self.guard.take() {
            Some(handle) => handle.join(),
            None => Ok(()),
        }
    }
}

impl Drop for WorkerGuard {
    fn drop(&mut self) {
        self.shutdown_signal.store(true, Ordering::Relaxed);
        match self.stop() {
            Ok(_) => (),
            Err(e) => println!("Failed to join worker thread. Error: {:?}", e),
        }
    }
}
