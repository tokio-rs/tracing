#![cfg(feature = "std")]
use std::thread;
use tracing::dispatch::{self, Dispatch};

mod support;

use self::support::*;

#[test]
fn global_scoped_is_overridable() {
    let (global, global_handle) = collector::mock()
        .event(event::msg("hello from thread 1"))
        .event(event::msg("hello from thread 2"))
        .event(event::msg("hello from thread 1 again"))
        .event(event::msg("hello from thread 1 a third time"))
        .done()
        .run_with_handle();
    let (scoped, scoped_handle) = collector::mock()
        .event(event::msg("hello scoped, it's me, thread 2"))
        .event(event::msg("hello scoped, it's me, thread 1"))
        .done()
        .run_with_handle();

    let _guard = dispatch::set_global_scoped(Dispatch::new(global));

    tracing::info!("hello from thread 1");

    let scoped = Dispatch::new(scoped);
    let scoped = thread::spawn(move || {
        tracing::info!("hello from thread 2");
        dispatch::with_default(&scoped, || {
            tracing::info!("hello scoped, it's me, thread 2");
        });
        scoped
    })
    .join()
    .expect("thread doesn't panic");

    tracing::info!("hello from thread 1 again");

    {
        let _guard = dispatch::set_default(&scoped);
        tracing::info!("hello scoped, it's me, thread 1");
    }

    tracing::info!("hello from thread 1 a third time");

    global_handle.assert_finished();
    scoped_handle.assert_finished();
}
