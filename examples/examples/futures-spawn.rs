#![deny(rust_2018_idioms)]

use futures::future::{self, Future};
use tracing::{debug, info, span, Level};
use tracing_futures::Instrument;

fn parent_task(subtasks: usize) -> impl Future<Item = (), Error = ()> {
    future::lazy(move || {
        info!("spawning subtasks...");
        let subtasks = (1..=subtasks)
            .map(|number| {
                debug!(message = "creating subtask;", number);
                subtask(number)
            })
            .collect::<Vec<_>>();
        future::join_all(subtasks)
    })
    .map(|result| {
        debug!("all subtasks completed");
        let sum: usize = result.into_iter().sum();
        info!(sum = sum);
    })
    .instrument(span!(Level::TRACE, "parent_task", subtasks))
}

fn subtask(number: usize) -> impl Future<Item = usize, Error = ()> {
    future::lazy(move || {
        info!("polling subtask...");
        Ok(number)
    })
    .instrument(span!(Level::TRACE, "subtask", number))
}

fn main() {
    use tracing_subscriber::{EnvFilter, FmtLayer, Layer, Registry};

    let subscriber = FmtLayer::default()
        .and_then(EnvFilter::try_new("futures-spawn=trace").unwrap())
        .with_subscriber(Registry::default());

    let _ = tracing::subscriber::set_global_default(subscriber);
    tokio::run(parent_task(10));
}
