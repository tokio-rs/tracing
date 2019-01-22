#[macro_use]
extern crate tokio_trace;
#[macro_use]
extern crate tokio_trace_derive;
extern crate env_logger;
extern crate tokio_trace_log;

use tokio_trace::field;

fn main() {
    env_logger::Builder::new().parse("trace").init();
    let subscriber = tokio_trace_log::TraceLogger::new();

    tokio_trace::subscriber::with_default(subscriber, || {
        let num: u64 = 1;

        let mut span = span!("Getting rec from another function.", number_of_recs = &num);
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
    format!("Wild Pink")
}
