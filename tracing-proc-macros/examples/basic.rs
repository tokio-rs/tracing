#[macro_use]
extern crate tracing;
#[macro_use]
extern crate tracing_proc_macros;
extern crate env_logger;
extern crate tracing_fmt;

use tracing::{field, Level};

fn main() {
    env_logger::Builder::new().parse("trace").init();
    let subscriber = tracing_fmt::FmtSubscriber::builder().full().finish();
    tracing::subscriber::with_default(subscriber, || {
        let num: u64 = 1;

        let span = span!(
            Level::TRACE,
            "Getting rec from another function.",
            number_of_recs = &num
        );
        span.enter(|| {
            let band = suggest_band();
            info!(
                { band_recommendation = field::display(&band) },
                "Got a rec."
            );
        });
    });
}

#[trace]
#[inline]
fn suggest_band() -> String {
    debug!("Suggesting a band.");
    format!("Wild Pink")
}
