use opentelemetry::sdk::propagation::B3Propagator;
use opentelemetry::{global, Context};
use std::collections::HashMap;
use tracing::span;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::subscribe::CollectorExt;
use tracing_subscriber::Registry;

fn make_request(_cx: Context) {
    // perform external request after injecting context
    // e.g. if there are request headers that impl `opentelemetry::propagation::Injector`
    // then `propagator.inject_context(cx, request.headers_mut())`
}

fn build_example_carrier() -> HashMap<String, String> {
    let mut carrier = HashMap::new();
    carrier.insert(
        "X-B3".to_string(),
        "4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-1".to_string(),
    );

    carrier
}

fn main() {
    // Set a format for propagating context. This MUST be provided as it is a no-op by default.
    global::set_text_map_propagator(B3Propagator::new());
    let subscriber = Registry::default().with(tracing_opentelemetry::layer());

    tracing::collect::with_default(subscriber, || {
        // Extract context from request headers
        let parent_context = global::get_text_map_propagator(|propagator| {
            propagator.extract(&build_example_carrier())
        });

        // Generate tracing span as usual
        let app_root = span!(tracing::Level::INFO, "app_start");

        // Assign parent trace from external context
        app_root.set_parent(&parent_context);

        // To include tracing context in client requests from _this_ app,
        // use `context` to extract the current OpenTelemetry context.
        make_request(app_root.context());
    });
}
