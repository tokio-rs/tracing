#![deny(rust_2018_idioms)]

use tracing::{debug, info, span, Level};
use tracing_attributes::instrument;

#[instrument]
#[inline]
fn suggest_band() -> String {
    debug!("Suggesting a band.");
    String::from("Wild Pink")
}

fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter("attrs_basic=trace")
        .finish();
    tracing::subscriber::with_default(subscriber, || {
        let num_recs = 1;

        let span = span!(Level::TRACE, "get_band_rec", ?num_recs);
        let _enter = span.enter();
        let band = suggest_band();
        info!(message = "Got a recommendation!", %band);
    });
}
