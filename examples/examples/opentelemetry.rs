use std::{thread, time::Duration};
use tracing::{span, trace, warn};
use tracing_attributes::instrument;
use tracing_subscriber::prelude::*;

#[instrument]
#[inline]
fn expensive_work() -> &'static str {
    span!(tracing::Level::INFO, "expensive_step_1")
        .in_scope(|| thread::sleep(Duration::from_millis(25)));
    span!(tracing::Level::INFO, "expensive_step_2")
        .in_scope(|| thread::sleep(Duration::from_millis(25)));

    "success"
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tracer, _uninstall) = opentelemetry_jaeger::new_pipeline()
        .with_service_name("report_example")
        .install()?;
    let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    tracing_subscriber::registry()
        .with(opentelemetry)
        .try_init()?;

    let root = span!(tracing::Level::INFO, "app_start", work_units = 2);
    let _enter = root.enter();

    let work_result = expensive_work();

    span!(tracing::Level::INFO, "faster_work")
        .in_scope(|| thread::sleep(Duration::from_millis(10)));

    warn!("About to exit!");
    trace!("status: {}", work_result);

    Ok(())
}
