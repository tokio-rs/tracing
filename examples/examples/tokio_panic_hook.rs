//! This example demonstrates that a custom panic hook can be used to log panic
//! messages even when panics are captured (such as when a Tokio task panics).
//!
//! This is essentially the same as the `panic_hook.rs` example, but modified to
//! spawn async tasks using the Tokio runtime rather than panicking in a
//! synchronous function. See the `panic_hook.rs` example for details on using
//! custom panic hooks.
#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .finish();

    // NOTE: Using `tracing` in a panic hook requires the use of the *global*
    // trace dispatcher (`tracing::subscriber::set_global_default`), rather than
    // the per-thread scoped dispatcher
    // (`tracing::subscriber::with_default`/`set_default`). With the scoped trace
    // dispatcher, the subscriber's thread-local context may already have been
    // torn down by unwinding by the time the panic handler is reached.
    tracing::subscriber::set_global_default(subscriber).unwrap();

    std::panic::set_hook(Box::new(|panic| {
        if let Some(location) = panic.location() {
            tracing::error!(
                message = %panic,
                panic.file = location.file(),
                panic.line = location.line(),
                panic.column = location.column(),
            );
        } else {
            tracing::error!(message = %panic);
        }
    }));

    // Spawn tasks to check the numbers from 1-10.
    let tasks = (0..10)
        .map(|i| tokio::spawn(check_number(i)))
        .collect::<Vec<_>>();
    futures::future::join_all(tasks).await;

    tracing::trace!("all tasks done");
}

#[tracing::instrument]
async fn check_number(x: i32) {
    tracing::trace!("checking number...");
    tokio::task::yield_now().await;

    if x % 2 == 0 {
        panic!("I don't work with even numbers!");
    }

    tracing::info!("number checks out!")
}
