use tracing_subscriber::{
    filter::{LevelFilter, Targets},
    fmt,
    prelude::*,
    Layer,
};

#[test]
fn noah_repro() {
    // reproduces https://github.com/tokio-rs/tracing/issues/1563#issuecomment-921363629
    let layer: Box<dyn Layer<_> + Send + Sync + 'static> =
        Box::new(fmt::layer().with_filter(Targets::new().with_target("hello", LevelFilter::DEBUG)));

    tracing_subscriber::registry().with(layer).init();

    for i in 0..1 {
        tracing::info!(target: "Hello", message = "Log hello", i);
    }
}
