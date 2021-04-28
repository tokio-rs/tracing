//! This example demonstrates how `tracing` events can be recorded from within a
//! panic hook, capturing the span context in which the program panicked.

fn main() {
    let collector = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .finish();

    // NOTE: Using `tracing` in a panic hook requires the use of the *global*
    // trace dispatcher (`tracing::collect::set_global_default`), rather than
    // the per-thread scoped dispatcher
    // (`tracing::collect::with_default`/`set_default`). With the scoped trace
    // dispatcher, the collector's thread-local context may already have been
    // torn down by unwinding by the time the panic handler is reached.
    tracing::collect::set_global_default(collector).unwrap();

    // Set a panic hook that records the panic as a `tracing` event at the
    // `ERROR` verbosity level.
    //
    // If we are currently in a span when the panic occurred, the logged event
    // will include the current span, allowing the context in which the panic
    // occurred to be recorded.
    std::panic::set_hook(Box::new(|panic| {
        // If the panic has a source location, record it as structured fields.
        if let Some(location) = panic.location() {
            // On nightly Rust, where the `PanicInfo` type also exposes a
            // `message()` method returning just the message, we could record
            // just the message instead of the entire `fmt::Display`
            // implementation, avoiding the duplciated location
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

    for i in 0..10 {
        check_number(i);
    }
}

#[tracing::instrument]
fn check_number(x: i32) {
    if x % 2 == 0 {
        panic!("I don't work with even numbers!");
    }
}
