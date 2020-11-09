use std::{
    io,
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};
use tracing::dispatch::Dispatch;

#[derive(Clone)]
pub(super) struct MultithreadedBench {
    start: Arc<Barrier>,
    end: Arc<Barrier>,
    dispatch: Dispatch,
}

impl MultithreadedBench {
    pub(super) fn new(dispatch: Dispatch) -> Self {
        Self {
            start: Arc::new(Barrier::new(5)),
            end: Arc::new(Barrier::new(5)),
            dispatch,
        }
    }

    pub(super) fn thread(&self, f: impl FnOnce() + Send + 'static) -> &Self {
        self.thread_with_setup(|start| {
            start.wait();
            f()
        })
    }

    pub(super) fn thread_with_setup(&self, f: impl FnOnce(&Barrier) + Send + 'static) -> &Self {
        let this = self.clone();
        thread::spawn(move || {
            let dispatch = this.dispatch.clone();
            tracing::dispatch::with_default(&dispatch, move || {
                f(&*this.start);
                this.end.wait();
            })
        });
        self
    }

    pub(super) fn run(&self) -> Duration {
        self.start.wait();
        let t0 = Instant::now();
        self.end.wait();
        t0.elapsed()
    }
}
/// A fake writer that doesn't actually do anything.
///
/// We want to measure the collectors's overhead, *not* the performance of
/// stdout/file writers. Using a no-op Write implementation lets us only measure
/// the collectors's overhead.
pub(super) struct NoWriter;

impl io::Write for NoWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl NoWriter {
    pub(super) fn new() -> Self {
        Self
    }
}
