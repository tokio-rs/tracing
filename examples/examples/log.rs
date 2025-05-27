//! NOTE: This is pre-release documentation for the upcoming tracing 0.2.0 ecosystem. For the
//! release examples, please see the `v0.1.x` branch instead.
fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    log::debug!("this is a log line");
    tracing::debug!("this is a tracing line");
}
