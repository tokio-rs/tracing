//! Regression test for <https://github.com/tokio-rs/tracing/issues/2228>.
//!
//! A scoped `set_default` must not install the global `log` logger as a side
//! effect. The `log` logger can only be installed once per process, so if the
//! scoped `set_default` installed it, a later global `init`/`try_init` would
//! fail with a `log::SetLoggerError` and panic.
//!
//! This test lives in its own binary because the `log` logger (and the global
//! default dispatcher) are process-global one-time resources.

#![cfg(all(feature = "std", feature = "tracing-log", feature = "fmt"))]

use tracing_subscriber::util::SubscriberInitExt;

#[test]
fn scoped_set_default_does_not_prevent_later_global_init() {
    // Set a scoped default. This must NOT install the global `log` logger.
    let subscriber = tracing_subscriber::fmt::Subscriber::new();
    let _guard = subscriber.set_default();
    drop(_guard);

    // A global init must still succeed: the `log` logger has not been
    // installed by the scoped `set_default` above.
    let subscriber = tracing_subscriber::fmt::Subscriber::new();
    assert!(
        subscriber.try_init().is_ok(),
        "try_init should succeed after a scoped set_default was dropped"
    );
}
