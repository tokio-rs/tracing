use tracing::{debug, info, span, Level};
use tracing_attributes::instrument;

#[instrument]
#[inline]
fn suggest_band() -> String {
    debug!("Suggesting a band.");
    format!("Wild Pink")
}

fn main() {
    let subscriber = tracing_fmt::FmtSubscriber::builder()
        .with_filter(tracing_fmt::filter::EnvFilter::from("basic=trace"))
        .finish();
    tracing::subscriber::with_default(subscriber, || {
        let num_recs = 1;

        let span = span!(Level::TRACE, "get_band_rec", ?num_recs);
        let _enter = span.enter();
        let band = suggest_band();
        info!(message = "Got a recomendation!", %band);
    });
}
