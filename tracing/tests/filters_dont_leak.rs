#![cfg(feature = "std")]

mod support;
use self::support::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn spans_dont_leak() {
    fn do_span() {
        let span = tracing::debug_span!("alice");
        let _e = span.enter();
    }

    let (collector, handle) = collector::mock()
        .named("spans/subscriber1")
        .with_filter(|_| false)
        .done()
        .run_with_handle();

    let _guard = tracing::collector::set_default(collector);

    do_span();

    let alice = span::mock().named("alice");
    let (subscriber2, handle2) = collector::mock()
        .named("spans/subscriber2")
        .with_filter(|_| true)
        .new_span(alice.clone())
        .enter(alice.clone())
        .exit(alice.clone())
        .drop_span(alice)
        .done()
        .run_with_handle();

    tracing::collector::with_default(subscriber2, || {
        println!("--- subscriber 2 is default ---");
        do_span()
    });

    println!("--- subscriber 1 is default ---");
    do_span();

    handle.assert_finished();
    handle2.assert_finished();
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn events_dont_leak() {
    fn do_event() {
        tracing::debug!("alice");
    }

    let (collector, handle) = collector::mock()
        .named("events/collector1")
        .with_filter(|_| false)
        .done()
        .run_with_handle();

    let _guard = tracing::collector::set_default(collector);

    do_event();

    let (collector2, handle2) = collector::mock()
        .named("events/collector2")
        .with_filter(|_| true)
        .event(event::mock())
        .done()
        .run_with_handle();

    tracing::collector::with_default(collector2, || {
        println!("--- collector 2 is default ---");
        do_event()
    });

    println!("--- collector 1 is default ---");

    do_event();

    handle.assert_finished();
    handle2.assert_finished();
}
