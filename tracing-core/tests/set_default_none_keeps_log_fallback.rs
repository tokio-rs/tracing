//! Regression test for <https://github.com/tokio-rs/tracing/issues/3642>.
//!
//! This lives in its own test binary because `has_been_set` is process-wide.
#![cfg(feature = "std")]

use tracing_core::{
    dispatcher::{self, Dispatch},
    subscriber::NoSubscriber,
};

#[test]
fn set_default_none_keeps_log_fallback() {
    assert!(!dispatcher::has_been_set());

    // Propagating "the current default" to another thread, as a library does
    // for a worker pool, hands out the no-op dispatch while none is set.
    let current = dispatcher::get_default(|current| current.clone());
    assert!(current.is::<NoSubscriber>());
    std::thread::spawn(move || {
        let _guard = dispatcher::set_default(&current);
    })
    .join()
    .unwrap();
    dispatcher::with_default(&Dispatch::none(), || {});
    assert!(
        !dispatcher::has_been_set(),
        "setting the no-op default dispatch must not count as setting a dispatcher"
    );

    // A real (scoped) subscriber still counts, even a no-op one.
    dispatcher::with_default(&Dispatch::new(NoSubscriber::default()), || {});
    assert!(dispatcher::has_been_set());
}
