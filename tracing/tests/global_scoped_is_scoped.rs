#![cfg(feature = "std")]
use std::thread;
use tracing::dispatch::{self, Dispatch};

mod support;

use self::support::*;

#[test]
fn global_scoped_is_scoped() {
    let (collector1, handle1) = collector::mock()
        .event(event::msg("hello from thread 1"))
        .event(event::msg("hello from thread 2"))
        .event(event::msg("hello from thread 1 a third time"))
        .done()
        .run_with_handle();

    let (collector2, handle2) = collector::mock()
        .event(event::msg("hello from thread 3"))
        .event(event::msg("hello from thread 1 again"))
        .done()
        .run_with_handle();

    dispatch::set_global_default(Dispatch::new(collector1)).expect("global default must be set");

    tracing::info!("hello from thread 1");
    thread::spawn(|| tracing::info!("hello from thread 2"))
        .join()
        .unwrap();

    {
        let _guard = dispatch::set_global_scoped(Dispatch::new(collector2));
        thread::spawn(|| tracing::info!("hello from thread 3"))
            .join()
            .unwrap();
        tracing::info!("hello from thread 1 again");
    }

    tracing::info!("hello from thread 1 a third time");

    handle1.assert_finished();
    handle2.assert_finished();
}
