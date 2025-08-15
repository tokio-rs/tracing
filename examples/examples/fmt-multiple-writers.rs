//! NOTE: This is pre-release documentation for the upcoming tracing 0.2.0 ecosystem. For the
//! release examples, please see the `v0.1.x` branch instead.
//!
//! An example demonstrating how `fmt::Subcriber` can write to multiple
//! destinations (in this instance, `stdout` and a file) simultaneously.
//! The stdout subscriber is configured with ANSI color codes, while the
//! file subscriber is configured without them.

#[path = "fmt/yak_shave.rs"]
mod yak_shave;

use std::{fs, io};
use tracing_subscriber::{fmt, subscribe::CollectExt, EnvFilter};

fn main() {
    let dir = tempfile::tempdir().expect("Failed to create tempdir");

    let file_appender = tracing_appender::rolling::hourly(dir.path(), "example.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let collector = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::TRACE.into()))
        .with(fmt::Subscriber::new().with_writer(io::stdout))
        .with(
            fmt::Subscriber::new()
                .with_ansi(false)
                .with_writer(non_blocking),
        );
    tracing::collect::set_global_default(collector).expect("Unable to set a global collector");

    let number_of_yaks = 3;
    // this creates a new event, outside of any spans.
    tracing::info!(number_of_yaks, "preparing to shave yaks");

    let number_shaved = yak_shave::shave_all(number_of_yaks);
    tracing::info!(
        all_yaks_shaved = number_shaved == number_of_yaks,
        "yak shaving completed."
    );

    // Print the log file contents to stdout since it's destroyed when the TempDir is dropped.
    fs::read_dir(dir.path())
        .expect("failed to read log dir")
        .map(|r| r.unwrap().path())
        .for_each(|log_file| {
            println!(
                "{:#^80}",
                format!(" BEGIN FILE CONTENTS: {} ", log_file.display())
            );
            print!("{}", std::fs::read_to_string(log_file).unwrap());
            println!("{:#^80}", " END FILE CONTENTS ");
        });
}
