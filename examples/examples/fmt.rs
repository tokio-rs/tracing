#![deny(rust_2018_idioms)]
use tracing::{info, Level};
use tracing_subscriber;

#[path = "fmt/yak_shave.rs"]
mod yak_shave;

fn main() {
    tracing_subscriber::fmt::Subscriber::builder()
        // all spans/events with a level higher than DEBUG (e.g, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::DEBUG)
        // sets this to be the default, global subscriber for this application.
        .init();

    let number_of_yaks = 3;
    info!(number_of_yaks, "preparing to shave yaks");

    let number_shaved = yak_shave::shave_all(number_of_yaks);
    info!(
        all_yaks_shaved = number_shaved == number_of_yaks,
        "yak shaving completed."
    );
}
