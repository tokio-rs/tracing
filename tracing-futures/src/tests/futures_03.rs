use futures::{future, sink, stream, FutureExt, SinkExt, StreamExt};
use tracing::subscriber::with_default;

use super::support::*;
use crate::*;

#[test]
fn stream_enter_exit_is_reasonable() {
    let (subscriber, handle) = subscriber::mock()
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .drop_span(span::mock().named("foo"))
        .run_with_handle();
    with_default(subscriber, || {
        stream::iter(&[1, 2, 3])
            .instrument(tracing::trace_span!("foo"))
            .for_each(|_| future::ready(()))
            .now_or_never()
            .unwrap();
    });
    handle.assert_finished();
}

#[test]
fn sink_enter_exit_is_reasonable() {
    let (subscriber, handle) = subscriber::mock()
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .enter(span::mock().named("foo"))
        .exit(span::mock().named("foo"))
        .drop_span(span::mock().named("foo"))
        .run_with_handle();
    with_default(subscriber, || {
        sink::drain()
            .instrument(tracing::trace_span!("foo"))
            .send(1u8)
            .now_or_never()
            .unwrap()
            .unwrap()
    });
    handle.assert_finished();
}
