use tracing_mock::layer;
use tracing::{
    subscriber::{with_default, Interest},
    Metadata, Subscriber,
};
use tracing_subscriber::{
    layer::{Context, Filter},
    prelude::*,
};

/// This filter disables all events and traces without short-circuiting the
/// `enabled` check.
struct DynFilter {}

impl<S> Filter<S> for DynFilter
where
    S: Subscriber,
{
    fn enabled(&self, _meta: &Metadata<'_>, _cx: &Context<'_, S>) -> bool {
        // We disable all callsites dynamically by returning `false` here.
        false
    }

    fn callsite_enabled(&self, _meta: &'static Metadata<'static>) -> Interest {
        // We make this filter dynamic for all callsites by returning `Interest::sometimes()` here.
        // If we returned `Interest::none()` then tracing could short-circuit and avoid
        // calling `enabled`.
        Interest::sometimes()
    }
}

#[test]
fn reentrant_different_event_is_filtered() {
    let filter = DynFilter {};
    let (layer, handle) = layer::mock().only().run_with_handle();
    let subscriber = tracing_subscriber::registry()
        .with(layer.with_filter(filter));

    fn event_and_value() -> &'static str {
        tracing::info!("This inner event breaks per-layer filtering");
        "whoops"
    }

    with_default(subscriber, || {
        tracing::info!("This event will be recorded: {}", event_and_value());
    });

    handle.assert_finished();
}

#[test]
fn reentrant_recursive_event_is_filtered() {
    let filter = DynFilter {};
    let (layer, handle) = layer::mock().only().run_with_handle();
    let subscriber = tracing_subscriber::registry()
        .with(layer.with_filter(filter));

    fn recursive_event(val: u64) -> u64 {
        if val > 1 {
            tracing::info!("counting down: {}", recursive_event(val-1))
        }
        val - 1
    }

    with_default(subscriber, || {
        _ = recursive_event(2);
    });

    handle.assert_finished();
}
