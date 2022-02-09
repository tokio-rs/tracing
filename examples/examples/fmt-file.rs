#![deny(rust_2018_idioms)]
#[path = "fmt/yak_shave.rs"]
mod yak_shave;

use std::{error::Error, fs::File};

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    use tracing_subscriber::EnvFilter;

    // In a real application, creating the log file may be more complex. For
    // example, the file path may be configured dynamically, the file name may
    // include a timestamp, an existing log file may be re-opened, etc.
    let file = File::create("fmt-file.log")?;

    tracing_subscriber::fmt()
        // use the default `RUST_LOG` environment variable to configure the
        // filter
        .with_env_filter(EnvFilter::from_default_env())
        // disable ANSI terminal formatting escape codes, since we are writing
        // to a file.
        .with_ansi(false)
        // configure the `fmt` collector to write to our log file, rather than
        // stdout.
        .with_writer(file)
        .init();

    let number_of_yaks = 3;
    // this creates a new event, outside of any spans.
    tracing::info!(number_of_yaks, "preparing to shave yaks");

    let number_shaved = yak_shave::shave_all(number_of_yaks);
    tracing::info!(
        all_yaks_shaved = number_shaved == number_of_yaks,
        "yak shaving completed."
    );

    Ok(())
}
