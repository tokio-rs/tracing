//! NOTE: This is pre-release documentation for the upcoming tracing 0.2.0 ecosystem. For the
//! release examples, please see the `v0.1.x` branch instead.
//! This is a example showing how information is scoped.
//!
//! You can run this example by running the following command in a terminal
//!
//! ```
//! cargo run --example spawny_thing
//! ```
#![deny(rust_2018_idioms)]

use futures::future::join_all;
use std::error::Error;
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
    info!(sum);
}

#[instrument]
async fn subtask(number: usize) -> usize {
    info!("polling subtask...");
    number
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init()?;
    parent_task(10).await;
    Ok(())
}
