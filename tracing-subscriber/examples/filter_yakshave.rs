#![deny(rust_2018_idioms)]

use tracing::Level;
use tracing_subscriber::prelude::*;

#[tracing::instrument]
fn shave(yak: usize) -> bool {
    tracing::debug!(
        message = "hello! I'm gonna shave a yak.",
        excitement = "yay!"
    );
    if yak == 3 {
        tracing::warn!(target: "yak_events", "could not locate yak!");
        false
    } else {
        tracing::trace!(target: "yak_events", "yak shaved successfully");
        true
    }
}

fn shave_all(yaks: usize) -> usize {
    let span = tracing::span!(Level::TRACE, "shaving_yaks", yaks_to_shave = yaks);
    let _enter = span.enter();

    tracing::info!("shaving yaks");

    let mut num_shaved = 0;
    for yak in 1..=yaks {
        let shaved = shave(yak);
        tracing::trace!(target: "yak_events", yak, shaved);

        if !shaved {
            tracing::error!(message = "failed to shave yak!", yak);
        } else {
            num_shaved += 1;
        }

        tracing::trace!(target: "yak_events", yaks_shaved = num_shaved);
    }

    num_shaved
}

fn main() {
    let subscriber = tracing_fmt::FmtSubscriber::builder()
        // Disable `tracing-fmt`'s filter implementation...
        .with_filter(tracing_fmt::filter::none())
        .finish()
        // and use `tracing-subscriber`'s filter layer instead.
        .with(tracing_subscriber::filter::Filter::from_default_env());

    tracing::subscriber::with_default(subscriber, || {
        let number_of_yaks = 3;
        tracing::debug!("preparing to shave {} yaks", number_of_yaks);

        let number_shaved = shave_all(number_of_yaks);

        tracing::debug!(
            message = "yak shaving completed.",
            all_yaks_shaved = number_shaved == number_of_yaks,
        );
    });
}
