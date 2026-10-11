#![cfg(feature = "std")]

use std::fmt;

use tracing::field::{debug, debug_alternate, display, display_alternate};
use tracing::subscriber::with_default;
use tracing::{event, info_span, Level};
use tracing_mock::*;

// Behaves similar to anyhow::Error.
struct ChainedError;

impl fmt::Display for ChainedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "outer: inner cause")
        } else {
            write!(f, "outer")
        }
    }
}

impl fmt::Debug for ChainedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("ChainedError")
                .field("msg", &"outer")
                .field("cause", &"inner cause")
                .finish()
        } else {
            write!(f, "ChainedError(outer)")
        }
    }
}

#[test]
fn display_alternate_in_event() {
    let (subscriber, handle) = subscriber::mock()
        .event(
            expect::event()
                .with_fields(expect::field("error").with_value(&display_alternate(&ChainedError))),
        )
        .only()
        .run_with_handle();
    with_default(subscriber, || {
        event!(Level::INFO, error = #%ChainedError);
    });
    handle.assert_finished();
}

#[test]
fn display_normal_in_event() {
    let (subscriber, handle) = subscriber::mock()
        .event(
            expect::event().with_fields(expect::field("error").with_value(&display(&ChainedError))),
        )
        .only()
        .run_with_handle();
    with_default(subscriber, || {
        event!(Level::INFO, error = %ChainedError);
    });
    handle.assert_finished();
}

#[test]
fn debug_alternate_in_event() {
    let (subscriber, handle) = subscriber::mock()
        .event(
            expect::event()
                .with_fields(expect::field("error").with_value(&debug_alternate(&ChainedError))),
        )
        .only()
        .run_with_handle();
    with_default(subscriber, || {
        event!(Level::INFO, error = #?ChainedError);
    });
    handle.assert_finished();
}

#[test]
fn debug_normal_in_event() {
    let (subscriber, handle) = subscriber::mock()
        .event(
            expect::event().with_fields(expect::field("error").with_value(&debug(&ChainedError))),
        )
        .only()
        .run_with_handle();
    with_default(subscriber, || {
        event!(Level::INFO, error = ?ChainedError);
    });
    handle.assert_finished();
}

#[test]
fn display_alternate_shorthand() {
    let (subscriber, handle) = subscriber::mock()
        .event(
            expect::event()
                .with_fields(expect::field("error").with_value(&display_alternate(&ChainedError))),
        )
        .only()
        .run_with_handle();
    with_default(subscriber, || {
        let error = ChainedError;
        event!(Level::INFO, #%error);
    });
    handle.assert_finished();
}

#[test]
fn alternate_in_span() {
    let span = expect::span().named("test");
    let (subscriber, handle) = subscriber::mock()
        .new_span(
            span.clone()
                .with_fields(expect::field("error").with_value(&display_alternate(&ChainedError))),
        )
        .drop_span(span)
        .only()
        .run_with_handle();
    with_default(subscriber, || {
        let _span = info_span!("test", error = #%ChainedError);
    });
    handle.assert_finished();
}
