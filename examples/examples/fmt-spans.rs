#![deny(rust_2018_idioms)]
use tracing::{debug, Level};
use tracing_subscriber::fmt;
#[path = "fmt/yak_shave.rs"]
mod yak_shave;

fn main() {
    let subscriber = fmt::Subscriber::builder()
        .with_max_level(Level::TRACE)
        .with_spans()
        .with_entry()
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let number_of_yaks = 3;
        debug!("preparing to shave {} yaks", number_of_yaks);

        let number_shaved = yak_shave::shave_all(number_of_yaks);

        debug!(
            message = "yak shaving completed.",
            all_yaks_shaved = number_shaved == number_of_yaks,
        );
    });
}
