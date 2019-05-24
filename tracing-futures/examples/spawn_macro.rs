extern crate futures;
extern crate tokio;
#[macro_use]
extern crate tokio_trace;
extern crate tokio_trace_fmt;
#[macro_use]
extern crate tokio_trace_futures;

use futures::future::{self, Future};

fn parent_task(how_many: usize) -> impl Future<Item = (), Error = ()> {
    future::lazy(move || {
        info!("spawning subtasks...");
        for i in 1..=how_many {
            debug!(message = "spawning subtask;", number = i);
            spawn!(subtask(i), number = i);
        }
        Ok(())
    })
    .map(|_| {
        info!("all subtasks spawned");
    })
}

fn subtask(number: usize) -> impl Future<Item = (), Error = ()> {
    future::lazy(move || {
        debug!("polling subtask...");
        Ok(number)
    })
    .map(|i| trace!(i = i))
}

fn main() {
    let subscriber = tokio_trace_fmt::FmtSubscriber::builder().full().finish();
    tokio_trace::subscriber::with_default(subscriber, || {
        tokio::run(parent_task(10));
    });
}
