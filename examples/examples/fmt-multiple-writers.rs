//! An example demonstrating how `fmt::Layer` can write to multiple
//! destinations (in this instance, `stdout` and a file) simultaneously.

#[path = "fmt/yak_shave.rs"]
mod yak_shave;

use std::io;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};

fn main() {
    let dir = tempfile::tempdir().expect("Failed to create tempdir");

    let file_appender = tracing_appender::rolling::hourly(dir, "example.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::TRACE.into()))
        .with(fmt::Layer::new().with_writer(io::stdout))
        .with(fmt::Layer::new().with_writer(non_blocking));
    tracing::subscriber::set_global_default(subscriber).expect("Unable to set a global subscriber");

    let number_of_yaks = 3;
    // this creates a new event, outside of any spans.
    tracing::info!(number_of_yaks, "preparing to shave yaks");

    let number_shaved = yak_shave::shave_all(number_of_yaks);
    tracing::info!(
        all_yaks_shaved = number_shaved == number_of_yaks,
        "yak shaving completed."
    );
}
