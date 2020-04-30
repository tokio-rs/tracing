#[macro_use]
extern crate tracing;

use opentelemetry::api::{HttpTextFormat, Provider};
use opentelemetry::{api, global};
use std::collections::HashMap;
use tracing_opentelemetry::{OpenTelemetryLayer, OpenTelemetrySpanExt};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

fn make_request(_span_context: api::SpanContext) {
    // perform external request after injecting context
    // e.g. if there are request headers that impl `opentelemetry::api::Carrier`
    // then `propagator.inject(span_context, request.headers_mut())`
}

fn build_example_carrier() -> HashMap<&'static str, String> {
    let mut carrier = HashMap::new();
    carrier.insert(
        "X-B3",
        "4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-1".to_string(),
    );

    carrier
}

fn main() {
    let tracer = global::trace_provider().get_tracer("example");
    let opentelemetry = OpenTelemetryLayer::with_tracer(tracer);
    let subscriber = Registry::default().with(opentelemetry);

    // Propagator can be swapped with trace context propagator binary propagator, etc.
    let propagator = api::B3Propagator::new(true);

    tracing::subscriber::with_default(subscriber, || {
        // Extract from request headers, or any type that impls `opentelemetry::api::Carrier`
        let parent_context = propagator.extract(&build_example_carrier());

        // Generate tracing span as usual
        let app_root = span!(tracing::Level::INFO, "app_start");

        // Assign parent trace from external context
        app_root.set_parent(parent_context);

        // To include tracing span context in client requests from _this_ app,
        // use `context` to extract the current OpenTelemetry span context.
        make_request(app_root.context());
    });
}
