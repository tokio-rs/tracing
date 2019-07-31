use futures::future::{self, Future};
use tracing::Level;
use tracing_futures::Instrument;

fn parent_task(subtasks: usize) -> impl Future<Item = (), Error = ()> {
    future::lazy(move || {
        tracing::info!("spawning subtasks...");
        let subtasks = (1..=subtasks)
            .map(|number| {
                tracing::debug!(message = "creating subtask;", number);
                subtask(number)
            })
            .collect::<Vec<_>>();
        future::join_all(subtasks)
    })
    .map(|result| {
        tracing::debug!("all subtasks completed");
        let sum: usize = result.into_iter().sum();
        tracing::info!(sum = sum);
    })
    .instrument(tracing::span!(Level::TRACE, "parent_task", subtasks))
}

fn subtask(number: usize) -> impl Future<Item = usize, Error = ()> {
    future::lazy(move || {
        tracing::info!("polling subtask...");
        Ok(number)
    })
    .instrument(tracing::span!(Level::TRACE, "subtask", number))
}

fn main() {
    let subscriber = tracing_fmt::FmtSubscriber::builder().finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
    tokio::run(parent_task(10));
}
