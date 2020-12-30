use opentelemetry::sdk::propagation::TraceContextPropagator;
use opentelemetry::{global, Context};
use std::collections::HashMap;
use tracing::span;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

fn make_request(_cx: Context) {
    // perform external request after injecting context
    // e.g. if there are request headers that impl `opentelemetry::propagation::Injector`
    // then `propagator.inject_context(cx, request.headers_mut())`
}

fn build_example_carrier() -> HashMap<String, String> {
    let mut carrier = HashMap::new();
    carrier.insert(
        "traceparent".to_string(),
        "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
    );

    carrier
}

fn main() {
    // Set a format for propagating context. This MUST be provided, as the default is a no-op.
    global::set_text_map_propagator(TraceContextPropagator::new());
    let subscriber = Registry::default().with(tracing_opentelemetry::layer());

    tracing::subscriber::with_default(subscriber, || {
        // Extract context from request headers
        let parent_context = global::get_text_map_propagator(|propagator| {
            propagator.extract(&build_example_carrier())
        });

        // Generate tracing span as usual
        let app_root = span!(tracing::Level::INFO, "app_start");

        // Assign parent trace from external context
        app_root.set_parent(parent_context);

        // To include tracing context in client requests from _this_ app,
        // use `context` to extract the current OpenTelemetry context.
        make_request(app_root.context());
    });
}
