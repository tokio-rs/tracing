use futures::future::{self, Future};

fn parent_task(how_many: usize) -> impl Future<Item = (), Error = ()> {
    future::lazy(move || {
        tracing::info!("spawning subtasks...");
        for number in 1..=how_many {
            tracing::debug!(message = "spawning subtask;", number);
            tracing_futures::spawn!(subtask(number), number);
        }
        Ok(())
    })
    .map(|_| {
        tracing::info!("all subtasks spawned");
    })
}

fn subtask(number: usize) -> impl Future<Item = (), Error = ()> {
    future::lazy(move || {
        tracing::debug!("polling subtask...");
        Ok(number)
    })
    .map(|i| tracing::trace!(i))
}

fn main() {
    let subscriber = tracing_fmt::FmtSubscriber::builder().finish();
    tracing::subscriber::with_default(subscriber, || {
        tokio::run(parent_task(10));
    });
}
