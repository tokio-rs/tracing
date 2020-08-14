#![deny(rust_2018_idioms)]
/// This is a example showing how `Layer` can be enabled or disabled by
/// by wrapping them with an `Option`. This example shows `fmt` and `json`
/// being toggled based on the `json` command line flag.
///
/// You can run this example by running the following command in a terminal
///
/// ```
/// cargo run --example toggle-layers -- json
/// ```
///
use clap::{App, Arg};
use tracing::info;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

#[path = "fmt/yak_shave.rs"]
mod yak_shave;

fn main() {
    let matches = App::new("fmt optional Example")
        .version("1.0")
        .arg(
            Arg::with_name("json")
                .help("Enabling json formatting of logs")
                .required(false)
                .takes_value(false),
        )
        .get_matches();

    let (json, plain) = if matches.is_present("json") {
        (Some(tracing_subscriber::fmt::layer().json()), None)
    } else {
        (None, Some(tracing_subscriber::fmt::layer()))
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
