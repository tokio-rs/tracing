#![deny(rust_2018_idioms)]

use tracing::Level;
use tracing_subscriber::prelude::*;

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

fn main() {
    let subscriber = tracing_fmt::FmtSubscriber::builder()
        // Disable `tracing-fmt`'s filter implementation...
        .with_filter(tracing_fmt::filter::none())
        .finish()
        // and use `tracing-subscriber`'s filter layer instead.
        .with(tracing_subscriber::filter::Filter::from_default_env());

    tracing::subscriber::with_default(subscriber, || {
        let yaks_to_shave = 3;
        let mut yaks_shaved = 0;
        tracing::debug!("preparing to shave {} yaks", yaks_to_shave);

        tracing::span!(Level::TRACE, "shaving_yaks", yaks_to_shave).in_scope(|| {
            tracing::info!("shaving yaks");

            for yak in 1..=yaks_to_shave {
                let span = tracing::span!(Level::TRACE, "shave", yak);
                let _e = span.enter();

                let shaved = shave(yak);
                tracing::trace!(shaved = shaved);

                if !shaved {
                    tracing::error!(message = "failed to shave yak!");
                } else {
                    yaks_shaved += 1;
                }

                tracing::trace!(yaks_shaved);
            }
        });

        tracing::debug!(
            message = "yak shaving completed.",
            all_yaks_shaved = yaks_shaved == yaks_to_shave,
        );
    });
}
