use super::*;
use tracing_subscriber::{filter, prelude::*, Layer};

fn filter<S>() -> filter::DynFilterFn<S> {
    // Use dynamic filter fn to disable interest caching and max-level hints,
    // allowing us to put all of these tests in the same file.
    filter::dynamic_filter_fn(|_, _| false)
}

#[test]
fn option_some() {
    let (layer, handle) = layer::mock().done().run_with_handle();
    let layer = Box::new(layer.with_filter(Some(filter())));

    let _guard = tracing_subscriber::registry().with(layer).set_default();

    for i in 0..2 {
        tracing::info!(i);
    }

    handle.assert_finished();
}

#[test]
fn option_none() {
    let (layer, handle) = layer::mock()
        .event(event::mock())
        .event(event::mock())
        .done()
        .run_with_handle();
    let layer = Box::new(layer.with_filter(None::<filter::DynFilterFn<_>>));

    let _guard = tracing_subscriber::registry().with(layer).set_default();

    for i in 0..2 {
        tracing::info!(i);
    }

    handle.assert_finished();
}
