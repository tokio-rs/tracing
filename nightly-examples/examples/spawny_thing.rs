#![feature(async_await)]
#![deny(rust_2018_idioms)]

/// This is a example showing how information is scoped.
///
/// You can run this example by running the following command in a terminal
///
/// ```
/// RUST_LOG=spawny_thing=trace cargo +nightly run --example spawny_thing
/// ```

use tokio;

use futures::future::join_all;
use tracing::{debug, info};
use tracing_attributes::instrument;

#[instrument]
async fn parent_task(subtasks: usize) {
    info!("spawning subtasks...");
    let subtasks = (1..=subtasks)
        .map(|number| {
            debug!(message = "creating subtask;", number);
            subtask(number)
        })
        .collect::<Vec<_>>();

    let result = join_all(subtasks).await;

    debug!("all subtasks completed");
    let sum: usize = result.into_iter().sum();
    info!(sum = sum);
}

#[instrument]
async fn subtask(number: usize) -> usize {
    info!("polling subtask... {}", number);
    number
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_fmt::FmtSubscriber::builder()
        .with_filter("trace".parse::<tracing_fmt::filter::EnvFilter>().unwrap())
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
    parent_task(10).await;
    Ok(())
}
