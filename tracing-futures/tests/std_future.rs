use std::{future::Future, pin::Pin, task};

use futures::FutureExt as _;
use tracing::Instrument;
use tracing::{subscriber::with_default, Level};
use tracing_mock::*;

#[test]
fn enter_exit_is_reasonable() {
    let (subscriber, handle) = subscriber::mock()
        .enter(span::expect().named("foo"))
        .exit(span::expect().named("foo"))
        .enter(span::expect().named("foo"))
        .exit(span::expect().named("foo"))
        .enter(span::expect().named("foo"))
        .exit(span::expect().named("foo"))
        .drop_span(span::expect().named("foo"))
        .only()
        .run_with_handle();
    with_default(subscriber, || {
        let future = PollN::new_ok(2).instrument(tracing::span!(Level::TRACE, "foo"));
        block_on_future(future).unwrap();
    });
    handle.assert_finished();
}

#[test]
fn error_ends_span() {
    let (subscriber, handle) = subscriber::mock()
        .enter(span::expect().named("foo"))
        .exit(span::expect().named("foo"))
        .enter(span::expect().named("foo"))
        .exit(span::expect().named("foo"))
        .enter(span::expect().named("foo"))
        .exit(span::expect().named("foo"))
        .drop_span(span::expect().named("foo"))
        .only()
        .run_with_handle();
    with_default(subscriber, || {
        let future = PollN::new_err(2).instrument(tracing::span!(Level::TRACE, "foo"));
        block_on_future(future).unwrap_err();
    });
    handle.assert_finished();
}

#[test]
fn span_on_drop() {
    #[derive(Clone, Debug)]
    struct AssertSpanOnDrop;

    impl Drop for AssertSpanOnDrop {
        fn drop(&mut self) {
            tracing::info!("Drop");
        }
    }

    struct Fut(Option<AssertSpanOnDrop>);

    impl Future for Fut {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, _: &mut task::Context<'_>) -> task::Poll<Self::Output> {
            self.set(Fut(None));
            task::Poll::Ready(())
        }
    }

    let subscriber = subscriber::mock()
        .enter(span::expect().named("foo"))
        .event(event::expect().at_level(Level::INFO))
        .exit(span::expect().named("foo"))
        .enter(span::expect().named("foo"))
        .exit(span::expect().named("foo"))
        .drop_span(span::expect().named("foo"))
        .enter(span::expect().named("bar"))
        .event(event::expect().at_level(Level::INFO))
        .exit(span::expect().named("bar"))
        .drop_span(span::expect().named("bar"))
        .only()
        .run();

    with_default(subscriber, || {
        // polled once
        Fut(Some(AssertSpanOnDrop))
            .instrument(tracing::span!(Level::TRACE, "foo"))
            .now_or_never()
            .unwrap();

        // never polled
        drop(Fut(Some(AssertSpanOnDrop)).instrument(tracing::span!(Level::TRACE, "bar")));
    });
}
