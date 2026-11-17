/// This example demonstrates usage same span on different threads with
/// spawn blocking.
///
/// Running:
///
/// ```
/// cargo run --example tokio_spawn_blocking
/// ```
///
use tracing::Instrument as _;
use std::future::Future;

#[tokio::main(worker_threads = 2)]
async fn main() {
    tracing_subscriber::fmt()
        .with_thread_ids(true)
        .try_init()
        .unwrap();

    let f1 = tokio::spawn(get_future().instrument(tracing::info_span!("future1")));
    let f2 = tokio::spawn(get_future().instrument(tracing::info_span!("future2")));

    f1.await.unwrap();
    f2.await.unwrap();
}

fn get_future() -> std::pin::Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async {
        tracing::info!("Sleeping");
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        // Getting copy span
        let span = tracing::Span::current();
        tokio::task::spawn_blocking(move || {
            let _guard = span.enter();
            tracing::info!("Sleeping in blocking");
            std::thread::sleep(std::time::Duration::from_secs(1));
            tracing::info!("Wakeup in blocking");
        })
        .await
        .unwrap();

        tracing::info!("Wakeup");
    })
}
