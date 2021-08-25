#![deny(rust_2018_idioms)]
/// This is a example showing how `Subscriber`s can be enabled or disabled by
/// by wrapping them with an `Option`. This example shows `fmt` and `json`
/// being toggled based on the `json` command line flag.
///
/// You can run this example by running the following command in a terminal
///
/// ```
/// cargo run --example toggle-subscribers -- --json
/// ```
///
use argh::FromArgs;
use tracing::info;
use tracing_subscriber::{subscribe::CollectExt, util::SubscriberInitExt};

#[path = "fmt/yak_shave.rs"]
mod yak_shave;

#[derive(FromArgs)]
/// Subscriber toggling example.
struct Args {
    /// enable JSON log format
    #[argh(switch, short = 'j')]
    json: bool,
}

fn main() {
    let args: Args = argh::from_env();

    let (json, plain) = if args.json {
        (Some(tracing_subscriber::fmt::subscriber().json()), None)
    } else {
        (None, Some(tracing_subscriber::fmt::subscriber()))
    };

    tracing_subscriber::registry().with(json).with(plain).init();

    let number_of_yaks = 3;
    // this creates a new event, outside of any spans.
    info!(number_of_yaks, "preparing to shave yaks");

    let number_shaved = yak_shave::shave_all(number_of_yaks);
    info!(
        all_yaks_shaved = number_shaved == number_of_yaks,
        "yak shaving completed."
    );
}
