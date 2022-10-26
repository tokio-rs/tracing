#![deny(rust_2018_idioms)]
use tracing::{error, info};
use tracing_journald::{Priority, PriorityMappings};
use tracing_subscriber::prelude::*;

#[path = "fmt/yak_shave.rs"]
mod yak_shave;

fn main() {
    let registry = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::subscriber().with_target(false));
    match tracing_journald::subscriber() {
        Ok(subscriber) => {
            registry
                .with(
                    subscriber
                        // We can tweak the mappings between the trace level and
                        // the journal priorities.
                        .with_priority_mappings(PriorityMappings {
                            info: Priority::Informational,
                            ..PriorityMappings::new()
                        }),
                )
                .init();
        }
        // journald is typically available on Linux systems, but nowhere else. Portable software
        // should handle its absence gracefully.
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
