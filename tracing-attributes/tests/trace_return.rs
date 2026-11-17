use std::convert::TryFrom;
use std::num::TryFromIntError;

use tracing::{subscriber::with_default, trace_return, Level};
use tracing_mock::{expect, subscriber};

#[trace_return(ret)]
fn ret() -> i32 {
    42
}

#[test]
fn emits_return_event_without_creating_span() {
    let (subscriber, handle) = subscriber::mock()
        .event(
            expect::event()
                .with_fields(expect::field("return").with_value(&tracing::field::debug(42)))
                .at_level(Level::INFO),
        )
        .only()
        .run_with_handle();

    with_default(subscriber, ret);
    handle.assert_finished();
}

#[test]
fn emits_event_in_current_span() {
    let parent = expect::span().named("parent");
    let event = expect::event()
        .with_fields(expect::field("return").with_value(&tracing::field::debug(42)))
        .with_ancestry(expect::has_contextual_parent("parent"));
    let (subscriber, handle) = subscriber::mock()
        .new_span(parent.clone())
        .enter(parent.clone())
        .event(event)
        .exit(parent.clone())
        .drop_span(parent)
        .only()
        .run_with_handle();

    with_default(subscriber, || {
        tracing::info_span!("parent").in_scope(ret);
    });
    handle.assert_finished();
}

#[trace_return(err(Debug, level = "info"))]
fn err_debug_info() -> Result<u8, TryFromIntError> {
    u8::try_from(1234)
}

#[test]
fn configures_error_format_and_level() {
    let error = u8::try_from(1234).unwrap_err();
    let (subscriber, handle) = subscriber::mock()
        .event(
            expect::event()
                .with_fields(
                    expect::field("error")
                        .with_value(&tracing::field::debug(error))
                        .only(),
                )
                .at_level(Level::INFO),
        )
        .only()
        .run_with_handle();

    with_default(subscriber, || err_debug_info().ok());
    handle.assert_finished();
}
