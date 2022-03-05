#![cfg(feature = "std")]
use std::thread;
use tracing::dispatch::{self, Dispatch};

mod support;

use self::support::*;

#[test]
fn global_scoped_poisoning() {
    let (collector, handle) = collector::mock()
        .event(event::msg("hello from thread 1"))
        .event(event::msg("hello from thread 2"))
        .event(event::msg("hello from thread 1 again"))
        .done()
        .run_with_handle();

    let _guard = dispatch::set_global_scoped(Dispatch::new(collector));

    tracing::info!("hello from thread 1");

    let joined = thread::spawn(|| {
        tracing::info!("hello from thread 2");
        panic!("fake panic lol");
    })
    .join();
    assert!(joined.is_err(), "it panicked");

    tracing::info!("hello from thread 1 again");

    handle.assert_finished();
}
