//! Tests for `on_register_dispatch` expectations in MockSubscriber and MockLayer.

use tracing_mock::subscriber;

#[test]
fn subscriber_on_register_dispatch() {
    let (subscriber, handle) = subscriber::mock().on_register_dispatch().run_with_handle();

    tracing::subscriber::with_default(subscriber, || {
        // The subscriber's on_register_dispatch is called when set as default
    });

    handle.assert_finished();
}

#[cfg(feature = "tracing-subscriber")]
#[test]
fn layer_on_register_dispatch() {
    use tracing_mock::layer;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let (layer, handle) = layer::mock().on_register_dispatch().run_with_handle();

    let _subscriber = tracing_subscriber::registry().with(layer).set_default();

    // The layer's on_register_dispatch is called when the subscriber is set as default
    drop(_subscriber);

    handle.assert_finished();
}

#[test]
fn subscriber_multiple_expectations() {
    let (subscriber, handle) = subscriber::mock()
        .on_register_dispatch()
        .event(tracing_mock::expect::event())
        .run_with_handle();

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("test event");
    });

    handle.assert_finished();
}

#[cfg(feature = "tracing-subscriber")]
#[test]
fn layer_multiple_expectations() {
    use tracing_mock::layer;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let (layer, handle) = layer::mock()
        .on_register_dispatch()
        .event(tracing_mock::expect::event())
        .run_with_handle();

    let _subscriber = tracing_subscriber::registry().with(layer).set_default();

    tracing::info!("test event");

    drop(_subscriber);
    handle.assert_finished();
}

#[cfg(feature = "tracing-subscriber")]
#[test]
#[should_panic(expected = "expected on_register_dispatch to be called")]
fn layer_on_register_dispatch_not_propagated() {
    use tracing::error;
    use tracing_mock::layer;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

    /// A layer that wraps another layer but does NOT propagate on_register_dispatch
    struct BadLayer<L> {
        inner: L,
    }

    impl<S, L> Layer<S> for BadLayer<L>
    where
        S: tracing_core::Subscriber,
        L: Layer<S>,
    {
        // Intentionally NOT implementing on_register_dispatch to test the failure case
        // The default implementation does nothing, so the inner layer won't receive the call

        fn on_event(
            &self,
            event: &tracing_core::Event<'_>,
            ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            self.inner.on_event(event, ctx);
        }
    }

    let (mock_layer, handle) = layer::named("inner")
        .on_register_dispatch()
        .run_with_handle();

    let bad_layer = BadLayer { inner: mock_layer };

    let _subscriber = tracing_subscriber::registry().with(bad_layer).set_default();

    // This event will be sent to the mock layer, which expects on_register_dispatch first
    error!("send an event");

    drop(_subscriber);

    handle.assert_finished();
}
