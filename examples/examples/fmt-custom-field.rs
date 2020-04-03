//! This example demonstrates overriding the way `tracing-subscriber`'s
//! `FmtSubscriber` formats fields on spans and events, using a closure.
#![deny(rust_2018_idioms)]
use tracing::Level;
use tracing_subscriber;

#[path = "fmt/yak_shave.rs"]
mod yak_shave;

fn main() {
    use tracing_subscriber::{fmt::format, prelude::*};

    // Format fields using the provided closure.
    let format = format::debug_fn(|writer, _field, value| {
        // We'll format _just_ the value of each field, ignoring the field name.
        write!(writer, "{:?}", value)
    })
    // Separate each field with a comma.
    // This method is provided by an extension trait in the
    // `tracing-subscriber` prelude.
    .delimited(", ");

    // Create a `fmt` subscriber that uses our custom event format, and set it
    // as the default.
    tracing_subscriber::fmt().fmt_fields(format).init();

    // Shave some yaks!
    let number_of_yaks = 3;
    // this creates a new event, outside of any spans.
    tracing::info!(number_of_yaks, "preparing to shave yaks");

    let number_shaved = yak_shave::shave_all(number_of_yaks);
    tracing::info!(
        all_yaks_shaved = number_shaved == number_of_yaks,
        "yak shaving completed."
    );
}
