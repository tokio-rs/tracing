#![deny(rust_2018_idioms)]
use tracing::{debug, error, info, span, trace, warn, Level};

#[tracing::instrument]
fn shave(yak: usize) -> bool {
    debug!(
        message = "hello! I'm gonna shave a yak.",
        excitement = "yay!"
    );
    if yak == 3 {
        warn!(target: "yak_events", "could not locate yak!");
        false
    } else {
        trace!(target: "yak_events", "yak shaved successfully");
        true
    }
}

fn shave_all(yaks: usize) -> usize {
    let span = span!(Level::TRACE, "shaving_yaks", yaks_to_shave = yaks);
    let _enter = span.enter();

    info!("shaving yaks");

    let mut num_shaved = 0;
    for yak in 1..=yaks {
        let shaved = shave(yak);
        trace!(target: "yak_events", yak, shaved);

        if !shaved {
            error!(message = "failed to shave yak!", yak);
        } else {
            num_shaved += 1;
        }

        trace!(target: "yak_events", yaks_shaved = num_shaved);
    }

    num_shaved
}

fn main() {
    use tracing_fmt::format;
    use tracing_subscriber::prelude::*;

    let formatter =
        // Construct a custom formatter for `Debug` fields
        format::debug_fn(|writer, field, value| write!(writer, "{}: {:?}", field, value))
            // Use the `tracing_subscriber::MakeFmtExt` trait to wrap the
            // formatter so that a delimiter is added between fields.
            .delimited(", ");

    let subscriber = tracing_fmt::FmtSubscriber::builder()
        .fmt_fields(formatter)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let number_of_yaks = 3;
        debug!("preparing to shave {} yaks", number_of_yaks);

        let number_shaved = shave_all(number_of_yaks);

        debug!(
            message = "yak shaving completed.",
            all_yaks_shaved = number_shaved == number_of_yaks,
        );
    });
}
