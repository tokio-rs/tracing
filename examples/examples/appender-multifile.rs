//! This example demonstrates the use of multiple files with
//! `tracing-appender`'s `RollingFileAppender`
//!
use tracing_appender::rolling;
use tracing_subscriber::fmt::writer::MakeWriterExt;

#[path = "fmt/yak_shave.rs"]
mod yak_shave;

fn main() {
    // Log all `tracing` events to files prefixed with `debug`. Since these
    // files will be written to very frequently, roll the log file every minute.
    let debug_file = rolling::minutely("./logs", "debug");
    // Log warnings and errors to a separate file. Since we expect these events
    // to occur less frequently, roll that file on a daily basis instead.
    let warn_file = rolling::daily("./logs", "warnings").with_max_level(tracing::Level::WARN);
    let all_files = debug_file.and(warn_file);

    tracing_subscriber::fmt()
        .with_writer(all_files)
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .init();

    yak_shave::shave_all(6);
    tracing::info!("sleeping for a minute...");

    std::thread::sleep(std::time::Duration::from_secs(60));

    tracing::info!("okay, time to shave some more yaks!");
    yak_shave::shave_all(10);
}
