#![deny(rust_2018_idioms)]
use std::io;
use tracing::debug;
use tracing_subscriber::{fmt::format::Format, fmt::time::ChronoUtc, fmt::Subscriber};

#[path = "fmt/yak_shave.rs"]
mod yak_shave;

fn main() {
    let format = Format::default()
        .with_timer(ChronoUtc::rfc3339())
        .with_ansi(false)
        .with_target(false)
        .json();
    let subscriber = Subscriber::builder()
        .with_writer(io::stdout)
        .on_event(format)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");

    let number_of_yaks = 3;
    debug!("preparing to shave {} yaks", number_of_yaks);

    let number_shaved = yak_shave::shave_all(number_of_yaks);

    debug!(
        message = "yak shaving completed.",
        all_yaks_shaved = number_shaved == number_of_yaks,
    );
}
