//! Demonstrates displaying events' source code locations with the `fmt`
//! subscriber.
#![deny(rust_2018_idioms)]
#[path = "fmt/yak_shave.rs"]
mod yak_shave;

fn main() {
    tracing_subscriber::fmt()
        // enable everything
        .with_max_level(tracing::Level::TRACE)
        // display source code file paths
        .with_file(true)
        // display source code line numbers
        .with_line_number(true)
        // disable targets
        .with_target(false)
        // sets this to be the default, global collector for this application.
        .init();

    let number_of_yaks = 3;
    // this creates a new event, outside of any spans.
    tracing::info!(number_of_yaks, "preparing to shave yaks");

    let number_shaved = yak_shave::shave_all(number_of_yaks);
    tracing::info!(
        all_yaks_shaved = number_shaved == number_of_yaks,
        "yak shaving completed."
    );
}
