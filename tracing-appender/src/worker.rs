use crate::inner::{InnerAppender, WriterFactory};
use crate::Rotation;
use chrono::{DateTime, Utc};
use crossbeam_channel::{Receiver, RecvError, TryRecvError};
use std::fmt::Debug;
use std::io::Write;
use std::{io, thread};

pub(crate) struct Worker<T: WriterFactory + Debug + Send + 'static> {
    inner: InnerAppender<T>,
    receiver: Receiver<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum WorkerState {
    Empty,
    Disconnected,
    Continue,
}

impl<T: WriterFactory + Debug + Send + 'static> Worker<T> {
    pub fn new(
        receiver: Receiver<Vec<u8>>,
        log_directory: &str,
        log_filename_prefix: &str,
        rotation: Rotation,
        writer_factory: T,
        now: DateTime<Utc>,
    ) -> io::Result<Self> {
        Ok(Self {
            inner: InnerAppender::new(
                &log_directory,
                &log_filename_prefix,
                rotation,
                writer_factory,
                now,
            )?,
            receiver,
        })
    }

    fn handle_recv(&mut self, result: &Result<Vec<u8>, RecvError>) -> io::Result<WorkerState> {
        match result {
            Ok(msg) => match self.inner.write(&msg) {
                Ok(_) => Ok(WorkerState::Continue),
                Err(e) => Err(e),
            },
            Err(_) => Ok(WorkerState::Disconnected),
        }
    }

    fn handle_try_recv(
        &mut self,
        result: &Result<Vec<u8>, TryRecvError>,
    ) -> io::Result<WorkerState> {
        match result {
            Ok(msg) => match self.inner.write(&msg) {
                Ok(_) => Ok(WorkerState::Continue),
                Err(e) => Err(e),
            },
            Err(e) => match e {
                TryRecvError::Empty => Ok(WorkerState::Empty),
                TryRecvError::Disconnected => Ok(WorkerState::Disconnected),
            },
        }
    }

    /// Blocks on the first recv of each batch of logs, unless the
    /// channel is disconnected. Afterwards, grabs as many logs as
    /// it can off the channel, buffers them and attempts a flush.
    pub fn work(&mut self) -> io::Result<WorkerState> {
        self.handle_recv(&self.receiver.recv())?;
        let mut worker_state = WorkerState::Continue;
        while worker_state == WorkerState::Continue {
            let try_recv_result = self.receiver.try_recv();
            let handle_result = self.handle_try_recv(&try_recv_result);
            worker_state = handle_result?;
        }
        self.inner.flush()?;
        Ok(worker_state)
    }

    /// Creates a worker thread that processes a channel until it's disconnected
    pub fn worker_thread(mut self) -> std::thread::JoinHandle<()> {
        thread::spawn(move || loop {
            let result = self.work();
            match &result {
                Ok(WorkerState::Continue) => {}
                Ok(WorkerState::Disconnected) => break,
                Ok(WorkerState::Empty) => {}
                Err(_) => {
                    // TODO: Expose a metric for IO Errors, or print to stderr
                }
            }
        })
    }
}
