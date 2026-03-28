//! An example demonstrating how to use per-layer filtering to direct different
//! levels of output to different destinations.
//!
//! This is a common real-world pattern: for example, logging DEBUG and above to
//! stdout for local development, while simultaneously logging WARN and above to
//! a file in JSON format for production monitoring.
//!
//! # Usage
//!
//! ```
//! cargo run --example per-layer-filter
//! ```
//!
//! The example writes human-readable output at DEBUG level to stdout, and
//! JSON-formatted output at WARN level to a temporary log file.

#[path = "fmt/yak_shave.rs"]
mod yak_shave;

use std::io;
use tracing_subscriber::{
    filter::{LevelFilter, Targets},
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Layer,
};

fn main() {
    let dir = tempfile::tempdir().expect("Failed to create tempdir");
    let log_path = dir.path().join("app.log");

    let file_appender = tracing_appender::rolling::never(dir.path(), "app.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // A pretty-printed stdout layer that captures DEBUG and above.
    let stdout_layer = fmt::layer()
        .with_writer(io::stdout)
        .pretty()
        .with_filter(LevelFilter::DEBUG);

    // A JSON file layer that captures WARN and above.
    let file_layer = fmt::layer().json().with_writer(non_blocking).with_filter(
        Targets::new()
            .with_target("per_layer_filter", LevelFilter::WARN)
            .with_target("yak_shave", LevelFilter::WARN),
    );

    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(file_layer)
        .init();

    let number_of_yaks = 3;
    tracing::info!(number_of_yaks, "preparing to shave yaks");

    let number_shaved = yak_shave::shave_all(number_of_yaks);
    tracing::info!(
        all_yaks_shaved = number_shaved == number_of_yaks,
        "yak shaving completed."
    );

    // Print the log file contents so we can see only WARN+ events were captured.
    // Drop the guard first to flush the non-blocking writer.
    drop(guard);
    eprintln!("\n--- Contents of {} ---", log_path.display());
    if let Ok(contents) = std::fs::read_to_string(&log_path) {
        eprint!("{contents}");
    }
}
