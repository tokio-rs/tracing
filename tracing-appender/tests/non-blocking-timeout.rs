use std::{
    io::{self, Write},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};
use tracing_appender::non_blocking::NonBlockingBuilder;

static BLOCK_IN_WORKER: AtomicBool = AtomicBool::new(false);
static BLOCK_DURATION_SECS: AtomicU64 = AtomicU64::new(3);

struct BlockingMemoryWriter {
    buffer: Vec<u8>,
}

impl BlockingMemoryWriter {
    fn new() -> Self {
        Self { buffer: Vec::new() }
    }
}

impl Write for BlockingMemoryWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if BLOCK_IN_WORKER.load(Ordering::Relaxed) {
            let block_secs = BLOCK_DURATION_SECS.load(Ordering::Relaxed);
            thread::sleep(Duration::from_secs(block_secs));
        }
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_shutdown_timeout_behavior() {
    let timeout = Duration::from_millis(300);
    let blocking_writer = BlockingMemoryWriter::new();

    let (mut non_blocking, guard) = NonBlockingBuilder::default()
        .shutdown_timeout(timeout)
        .finish(blocking_writer);

    non_blocking.write_all(b"test data\n").unwrap();

    thread::sleep(Duration::from_millis(50));
    BLOCK_IN_WORKER.store(true, Ordering::Relaxed);
    non_blocking.write_all(b"blocking data\n").unwrap();

    let start = Instant::now();
    drop(guard);
    let elapsed = start.elapsed();

    assert!(
        elapsed >= timeout,
        "Shutdown completed before timeout: {:?}, expected at least {:?}",
        elapsed,
        timeout
    );
}
