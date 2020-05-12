use opentelemetry::api::Provider;
use opentelemetry::sdk;
use std::{io, thread, time::Duration};
use tracing::{span, trace, warn};
use tracing_attributes::instrument;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

#[instrument]
#[inline]
fn expensive_work() -> &'static str {
    span!(tracing::Level::INFO, "expensive_step_1")
        .in_scope(|| thread::sleep(Duration::from_millis(25)));
    span!(tracing::Level::INFO, "expensive_step_2")
        .in_scope(|| thread::sleep(Duration::from_millis(25)));

    "success"
}

fn init_tracer() -> Result<(), Box<dyn std::error::Error>> {
    let exporter = opentelemetry_jaeger::Exporter::builder()
        .with_agent_endpoint("127.0.0.1:6831".parse().unwrap())
        .with_process(opentelemetry_jaeger::Process {
            service_name: "report_example".to_string(),
            tags: Vec::new(),
        })
        .init()
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
    let provider = sdk::Provider::builder()
        .with_simple_exporter(exporter)
        .with_config(sdk::Config {
            default_sampler: Box::new(sdk::Sampler::Always),
            ..Default::default()
        })
        .build();
    let tracer = provider.get_tracer("tracing");

    let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    tracing_subscriber::registry()
        .with(opentelemetry)
        .try_init()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracer()?;

    let root = span!(tracing::Level::INFO, "app_start", work_units = 2);
    let _enter = root.enter();

    let work_result = expensive_work();

    span!(tracing::Level::INFO, "faster_work")
        .in_scope(|| thread::sleep(Duration::from_millis(10)));

    warn!("About to exit!");
    trace!("status: {}", work_result);

    Ok(())
}
