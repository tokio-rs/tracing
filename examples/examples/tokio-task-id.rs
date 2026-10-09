#![deny(rust_2018_idioms)]
/// This is a example showing how tokio task id can be displayed when
/// formatting events with `tracing_subscriber::fmt`. This is useful
/// as the task ID makes it easier to trace activity back to the
/// specific asynchronous task that produced the log.
///
/// You can run this example by running the following command in a terminal
///
/// ```
/// cargo run --features tokio-task-id --example tokio-task-id
/// ```
///
/// Example output:
///
/// ```not_rust
/// 2026-04-15T20:27:00.900174Z  INFO Id(11) tokio_task_id: i=1
/// 2026-04-15T20:27:00.900224Z  INFO Id(12) tokio_task_id: i=1
/// 2026-04-15T20:27:00.902561Z  INFO Id(12) tokio_task_id: i=2
/// ```
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        // enable tokio task id to be emitted
        .with_tokio_task_ids(true)
        .init();

    let do_one_work = async {
        for i in 1..10 {
            info!(i);
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    };

    let do_other_work = async {
        for i in 1..20 {
            info!(i);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    };

    let task_one = tokio::task::spawn(do_one_work);
    let task_two = tokio::task::spawn(do_other_work);

    let _ = task_one.await;
    let _ = task_two.await;
}
