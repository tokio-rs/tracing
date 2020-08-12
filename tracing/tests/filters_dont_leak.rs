#![cfg(feature = "std")]

mod support;
use self::support::*;

#[test]
fn spans_dont_leak() {
    fn do_span() {
        tracing::debug_span!("alice");
    }

    let (subscriber, handle) = subscriber::mock()
        .with_filter(|_| false)
        .done()
        .run_with_handle();

    let _guard = tracing::subscriber::set_default(subscriber);

    do_span();

    let (subscriber2, handle2) = subscriber::mock()
        .with_filter(|_| true)
        .new_span(span::mock().named("alice"))
        .drop_span(span::mock().named("alice"))
        .done()
        .run_with_handle();

    tracing::subscriber::with_default(subscriber2, do_span);

    do_span();

    handle.assert_finished();
    handle2.assert_finished();
}

#[test]
fn events_dont_leak() {
    fn do_event() {
        tracing::debug!("alice");
    }

    let (subscriber, handle) = subscriber::mock()
        .with_filter(|_| false)
        .done()
        .run_with_handle();

    let _guard = tracing::subscriber::set_default(subscriber);

    do_event();

    let (subscriber2, handle2) = subscriber::mock()
        .with_filter(|_| true)
        .event(event::mock())
        .done()
        .run_with_handle();

    tracing::subscriber::with_default(subscriber2, do_event);

    do_event();

    handle.assert_finished();
    handle2.assert_finished();
}
