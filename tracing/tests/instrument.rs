// These tests require the thread-local scoped dispatcher, which only works when
// we have a standard library. The behaviour being tested should be the same
// with the standard lib disabled.
#![cfg(feature = "std")]

use std::{future::Future, pin::Pin, task};

use futures::FutureExt as _;
use tracing::{subscriber::with_default, Instrument as _, Level};
use tracing_mock::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
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
