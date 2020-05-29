#![deny(rust_2018_idioms)]
/// This is a example showing how information is scoped with tokio's
/// `task::spawn`.
///
/// You can run this example by running the following command in a terminal
///
/// ```
/// cargo run --example tokio-spawny-thing
/// ```
use futures::future::try_join_all;
use tracing::{debug, info, instrument, span, Level};
use tracing_futures::Instrument;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[instrument]
async fn parent_task(subtasks: usize) -> Result<(), Error> {
    info!("spawning subtasks...");
    let mut subtasks = (1..=subtasks)
        .map(|number| {
            let span = span!(Level::INFO, "subtask", %number);
            debug!(message = "creating subtask;", number);
            tokio::spawn(subtask(number).instrument(span))
        })
        .collect::<Vec<_>>();

    // the returnable error would be if one of the subtasks panicked.
    let sum: usize = try_join_all(subtasks).await?.iter().sum();
    info!(%sum, "all subtasks completed; calculated sum");
    Ok(())
}

async fn subtask(number: usize) -> usize {
    info!(%number, "polling subtask");
    subsubtask(number).await
}

#[instrument]
async fn subsubtask(number: usize) -> usize {
    info!(%number, "polling subsubtask");
    number
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init()?;
    parent_task(10).await?;
    Ok(())
}
