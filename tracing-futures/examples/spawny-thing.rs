#![deny(rust_2018_idioms)]

use futures::future::{self, Future};
use tracing::{debug, info, span, Level};
use tracing_futures::Instrument;

fn parent_task(how_many: usize) -> impl Future<Item = (), Error = ()> {
    future::lazy(move || {
        info!("spawning subtasks...");
        let subtasks = (1..=how_many)
            .map(|i| {
                debug!(message = "creating subtask;", number = i);
                subtask(i)
            })
            .collect::<Vec<_>>();
        future::join_all(subtasks)
    })
    .map(|result| {
        debug!("all subtasks completed");
        let sum: usize = result.into_iter().sum();
        info!(sum = sum);
    })
    .instrument(span!(Level::TRACE, "parent_task", subtasks = how_many))
}

fn subtask(number: usize) -> impl Future<Item = usize, Error = ()> {
    future::lazy(move || {
        info!("polling subtask...");
        Ok(number)
    })
    .instrument(span!(Level::TRACE, "subtask", number = number))
}

fn main() {
    let subscriber = tracing_fmt::FmtSubscriber::builder().finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
    tokio::run(parent_task(10));
}
