#![cfg(feature = "registry")]
use tracing::level_filters::LevelFilter;
use tracing::Subscriber;
use tracing_subscriber::prelude::*;

#[test]
fn just_empty_vec() {
    // Just a None means everything is off
    let subscriber = tracing_subscriber::registry().with(Vec::<LevelFilter>::new());
    assert_eq!(subscriber.max_level_hint(), Some(LevelFilter::OFF));
}

#[test]
fn layer_and_empty_vec() {
    let subscriber = tracing_subscriber::registry()
        .with(LevelFilter::INFO)
        .with(Vec::<LevelFilter>::new());
    assert_eq!(subscriber.max_level_hint(), Some(LevelFilter::INFO));
}

#[test]
fn on_register_dispatch_is_called() {
    let (inner_layer_0, inner_handle_0) = tracing_mock::layer::named("inner0")
        .on_register_dispatch()
        .run_with_handle();
    let (inner_layer_1, inner_handle_1) = tracing_mock::layer::named("inner0")
        .on_register_dispatch()
        .run_with_handle();

    let subscriber = tracing_subscriber::registry().with(vec![inner_layer_0, inner_layer_1]);
    tracing::subscriber::with_default(subscriber, || {});

    inner_handle_0.assert_finished();
    inner_handle_1.assert_finished();
}
