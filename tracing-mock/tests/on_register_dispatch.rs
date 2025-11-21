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
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

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
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

    let (layer, handle) = layer::mock()
        .on_register_dispatch()
        .event(tracing_mock::expect::event())
        .run_with_handle();

    let _subscriber = tracing_subscriber::registry().with(layer).set_default();

    tracing::info!("test event");

    drop(_subscriber);
    handle.assert_finished();
}
