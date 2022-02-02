use super::*;
use tracing_subscriber::prelude::*;

#[test]
fn not() {
    let layer1 = layer::mock().done().run();
    let layer2 = layer::mock().done().run();

    let no_filter = filter::filter_fn(|_| false);
    let yes_filter = filter::filter_fn(|_| true);
    let _subscriber = tracing_subscriber::registry()
        .with(layer1.with_filter(no_filter))
        .with(layer2.with_filter(yes_filter.clone()))
        .set_default();

    assert!(tracing::enabled!(tracing::Level::INFO));
    assert!(!tracing::fully_enabled!(tracing::Level::INFO));
}

#[test]
fn yes() {
    let layer1 = layer::mock().done().run();
    let layer2 = layer::mock().done().run();

    let yes_filter = filter::filter_fn(|_| true);
    let _subscriber = tracing_subscriber::registry()
        .with(layer1.with_filter(yes_filter.clone()))
        .with(layer2.with_filter(yes_filter))
        .set_default();

    assert!(tracing::enabled!(tracing::Level::INFO));
    assert!(tracing::fully_enabled!(tracing::Level::INFO));
}
