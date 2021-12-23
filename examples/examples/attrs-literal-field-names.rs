#![deny(rust_2018_idioms)]

use tracing::{debug, span, Level};
use tracing_attributes::instrument;

#[instrument]
#[inline]
fn suggest_band() -> String {
    debug!("Suggesting a band.");
    String::from("Wild Pink")
}

fn main() {
    let collector = tracing_subscriber::fmt()
        .with_env_filter("attrs_literal_field_names=trace")
        .finish();
    tracing::collect::with_default(collector, || {
        let span = span!(Level::TRACE, "get_band_rec", "guid:x-request-id" = "abcdef");
        let _enter = span.enter();
        suggest_band();
    });
}
