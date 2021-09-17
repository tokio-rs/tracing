use super::*;
use std::sync::Arc;
use tracing_subscriber::{
    filter::{LevelFilter, Targets},
    prelude::*,
    Layer,
};

fn layer() -> (ExpectLayer, subscriber::MockHandle) {
    layer::mock()
        .event(event::mock().with_fields(field::mock("i").with_value(&0)))
        .event(event::mock().with_fields(field::mock("i").with_value(&1)))
        .done()
        .run_with_handle()
}

/// reproduces https://github.com/tokio-rs/tracing/issues/1563#issuecomment-921363629
#[test]
fn box_works() {
    let (layer, handle) = layer();
    let layer =
        Box::new(layer.with_filter(Targets::new().with_target("hello", LevelFilter::DEBUG)));

    let _guard = tracing_subscriber::registry().with(layer).set_default();

    for i in 0..1 {
        tracing::info!(target: "Hello", message = "hello", i);
    }

    handle.assert_finished();
}

/// the same as `box_works` but with a type-erased `Box`.
#[test]
fn dyn_box_works() {
    let (layer, handle) = layer();
    let layer: Box<dyn Layer<_> + Send + Sync + 'static> =
        Box::new(layer.with_filter(Targets::new().with_target("hello", LevelFilter::DEBUG)));

    let _guard = tracing_subscriber::registry().with(layer).set_default();

    for i in 0..1 {
        tracing::info!(target: "Hello", message = "hello", i);
    }

    handle.assert_finished();
}

/// the same as `box_works` but with an `Arc`.
#[test]
fn arc_works() {
    let (layer, handle) = layer();
    let layer =
        Box::new(layer.with_filter(Targets::new().with_target("hello", LevelFilter::DEBUG)));

    let _guard = tracing_subscriber::registry().with(layer).set_default();

    for i in 0..1 {
        tracing::info!(target: "Hello", message = "hello", i);
    }

    handle.assert_finished();
}

/// the same as `box_works` but with a type-erased `Arc`.
#[test]
fn dyn_arc_works() {
    let (layer, handle) = layer();
    let layer: Arc<dyn Layer<_> + Send + Sync + 'static> =
        Arc::new(layer.with_filter(Targets::new().with_target("hello", LevelFilter::DEBUG)));

    let _guard = tracing_subscriber::registry().with(layer).set_default();

    for i in 0..1 {
        tracing::info!(target: "Hello", message = "hello", i);
    }

    handle.assert_finished();
}
