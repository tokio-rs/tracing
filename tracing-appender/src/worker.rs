use crossbeam_channel::{Receiver, RecvError, TryRecvError};
use std::fmt::Debug;
use std::io::Write;
use std::{io, thread};
use tracing_subscriber::fmt::MakeWriter;

pub(crate) struct Worker<T: MakeWriter + Send + Sync + 'static> {
    writer: T,
    receiver: Receiver<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum WorkerState {
    Empty,
    Disconnected,
    Continue,
}

impl<T: MakeWriter + Send + Sync + 'static> Worker<T> {
    pub(crate) fn new(receiver: Receiver<Vec<u8>>, writer: T) -> Worker<T> {
        Self { writer, receiver }
    }

    fn handle_recv(&mut self, result: &Result<Vec<u8>, RecvError>) -> io::Result<WorkerState> {
        match result {
            Ok(msg) => {
                self.writer.make_writer().write(&msg)?;
                Ok(WorkerState::Continue)
            }
            Err(_) => Ok(WorkerState::Disconnected),
        }
    }

    fn handle_try_recv(
        &mut self,
        result: &Result<Vec<u8>, TryRecvError>,
    ) -> io::Result<WorkerState> {
        match result {
            Ok(msg) => match self.writer.make_writer().write(&msg) {
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
    pub(crate) fn work(&mut self) -> io::Result<WorkerState> {
        self.handle_recv(&self.receiver.recv())?;
        let mut worker_state = WorkerState::Continue;
        while worker_state == WorkerState::Continue {
            let try_recv_result = self.receiver.try_recv();
            let handle_result = self.handle_try_recv(&try_recv_result);
            worker_state = handle_result?;
        }
        self.writer.make_writer().flush()?;
        Ok(worker_state)
    }

    /// Creates a worker thread that processes a channel until it's disconnected
    pub(crate) fn worker_thread(mut self) -> std::thread::JoinHandle<()> {
        thread::spawn(move || loop {
            let result = self.work();
            match &result {
                Ok(WorkerState::Continue) | Ok(WorkerState::Empty) => {}
                Ok(WorkerState::Disconnected) => break,
                Err(_) => {
                    // TODO: Expose a metric for IO Errors, or print to stderr
                }
            }
        })
    }
}
