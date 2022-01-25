use opentelemetry::global;
use opentelemetry::sdk::propagation::TraceContextPropagator;
use std::collections::HashMap;
use std::{error::Error, thread, time::Duration};
use tracing::{debug, info, span, trace, warn};
use tracing_attributes::instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::prelude::*;

#[instrument]
#[inline]
fn expensive_work() -> &'static str {
    span!(tracing::Level::INFO, "expensive_step_1").in_scope(|| {
        debug!("doing expensive step 1");
        thread::sleep(Duration::from_millis(25));
    });
    span!(tracing::Level::INFO, "expensive_step_2").in_scope(|| {
        debug!("doing expensive step 2");
        thread::sleep(Duration::from_millis(25))
    });

    "success"
}

/// Does some work inside a span (supposedly) propagated from a remote trace (we
/// actually fake this for the purposes of the example).
#[instrument]
fn in_remote_span() {
    // Extract context from request headers
    let parent_context =
        global::get_text_map_propagator(|propagator| propagator.extract(&build_example_carrier()));
    tracing::Span::current().set_parent(parent_context);

    info!("doing some work in a remote span...");
    thread::sleep(Duration::from_millis(10));
    info!("...done");
}

fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    // Set a format for propagating context. This MUST be provided, as the default is a no-op.
    global::set_text_map_propagator(TraceContextPropagator::new());
    let (tracer, _uninstall) = opentelemetry_jaeger::new_pipeline()
        .with_service_name("report_example")
        .install()?;
    let opentelemetry = tracing_opentelemetry::subscriber().with_tracer(tracer);
    let fmt = tracing_subscriber::fmt::subscriber().event_format(
        tracing_opentelemetry::fmt()
            // Display OpenTelemetry span IDs when logging events
            .with_span_ids(true)
            // Display any remote OpenTelemetry parent spans when formatting the
            // event context.
            .with_remote_parents(true)
            // Like `tracing_subscriber`'s default formatters, we can enable and
            // disable parts of the log line, like targets...
            .with_target(false),
    );
    tracing_subscriber::registry()
        .with(fmt)
        .with(opentelemetry)
        .try_init()?;

    let root = span!(tracing::Level::INFO, "app_start", work_units = 2);
    let _enter = root.enter();

    let work_result = expensive_work();

    in_remote_span();

    warn!("About to exit!");
    trace!("status: {}", work_result);

    Ok(())
}

fn build_example_carrier() -> HashMap<String, String> {
    let mut carrier = HashMap::new();
    carrier.insert(
        "traceparent".to_string(),
        "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
    );

    carrier
}
