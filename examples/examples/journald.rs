#![deny(rust_2018_idioms)]
use tracing::{error, info};
use tracing_subscriber::prelude::*;

#[path = "fmt/yak_shave.rs"]
mod yak_shave;

#[cfg(not(target_os = "linux"))]
fn main() {
    error!("tracing-journald only works on Linux");
}

#[cfg(target_os = "linux")]
fn main() {
    let registry = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::subscriber().with_target(false));
    match tracing_journald::subscriber() {
        Ok(subscriber) => {
            registry.with(subscriber).init();
        }
        // Linux systems which do not use systemd will likely not have journald either;
        // software should handle this gracefully and fallback to another subscriber.
        Err(e) => {
            registry.init();
            error!("couldn't connect to journald: {}", e);
        }
    }

    let number_of_yaks = 3;
    // this creates a new event, outside of any spans.
    info!(number_of_yaks, "preparing to shave yaks");

    let number_shaved = yak_shave::shave_all(number_of_yaks);
    info!(
        all_yaks_shaved = number_shaved == number_of_yaks,
        "yak shaving completed."
    );
}
