// These tests include field filters with no targets, so they have to go in a
// separate file.
#![cfg(feature = "env-filter")]

use tracing::{self, collect::with_default, Level};
use tracing_mock::*;
use tracing_subscriber::{filter::EnvFilter, prelude::*};

#[test]
fn same_length_targets() {
    let filter: EnvFilter = "foo=trace,bar=trace".parse().expect("filter should parse");
    let (subscriber, finished) = collector::mock()
        .event(event::expect().at_level(Level::TRACE))
        .event(event::expect().at_level(Level::TRACE))
        .only()
        .run_with_handle();
    let subscriber = subscriber.with(filter);

    with_default(subscriber, || {
        tracing::trace!(target: "foo", "foo");
        tracing::trace!(target: "bar", "bar");
    });

    finished.assert_finished();
}

#[test]
fn same_num_fields_event() {
    let filter: EnvFilter = "[{foo}]=trace,[{bar}]=trace"
        .parse()
        .expect("filter should parse");
    let (subscriber, finished) = collector::mock()
        .event(
            event::expect()
                .at_level(Level::TRACE)
                .with_fields(field::expect("foo")),
        )
        .event(
            event::expect()
                .at_level(Level::TRACE)
                .with_fields(field::expect("bar")),
        )
        .only()
        .run_with_handle();
    let subscriber = subscriber.with(filter);
    with_default(subscriber, || {
        tracing::trace!(foo = 1);
        tracing::trace!(bar = 3);
    });

    finished.assert_finished();
}

#[test]
fn same_num_fields_and_name_len() {
    let filter: EnvFilter = "[foo{bar=1}]=trace,[baz{boz=1}]=trace"
        .parse()
        .expect("filter should parse");
    let (subscriber, finished) = collector::mock()
        .new_span(
            span::expect()
                .named("foo")
                .at_level(Level::TRACE)
                .with_field(field::expect("bar")),
        )
        .new_span(
            span::expect()
                .named("baz")
                .at_level(Level::TRACE)
                .with_field(field::expect("boz")),
        )
        .only()
        .run_with_handle();
    let subscriber = subscriber.with(filter);
    with_default(subscriber, || {
        tracing::trace_span!("foo", bar = 1);
        tracing::trace_span!("baz", boz = 1);
    });

    finished.assert_finished();
}
