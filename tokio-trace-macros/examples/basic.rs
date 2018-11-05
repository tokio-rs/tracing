#[macro_use]
extern crate tokio_trace;
#[macro_use]
extern crate tokio_trace_macros;
extern crate env_logger;
extern crate tokio_trace_log;

use tokio_trace::Level;

fn main() {
    env_logger::Builder::new().parse("trace").init();
    let subscriber = tokio_trace_log::TraceLogger::new();

    tokio_trace::Dispatch::to(subscriber).as_default(|| {
        let num = 1;

        let mut span = span!("Getting rec from another function.", number_of_recs = &num);
        span.enter(|| {
            let band = suggest_band();
            event!(Level::Info, { band_recommendation = &band }, "Got a rec.");
        });
    });
}

#[trace]
#[inline]
fn suggest_band() -> String {
    format!("Wild Pink")
}
