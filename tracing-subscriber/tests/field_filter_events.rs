// Note: this test and the `field_filter_spans` test are defined in separate
// files to avoid possible flakiness due to the tests interfering with the
// process-global filter state.
// TODO(eliza): would be nice if this wasn't necessary...
mod support;
use self::support::*;
use tracing::{self, collect::with_default, Level};
use tracing_subscriber::{filter::EnvFilter, prelude::*};

#[test]
fn field_filter_events() {
    let filter: EnvFilter = "[{thing}]=debug".parse().expect("filter should parse");
    let (subscriber, finished) = collector::mock()
        .event(
            event::mock()
                .at_level(Level::INFO)
                .with_fields(field::mock("thing")),
        )
        .event(
            event::mock()
                .at_level(Level::DEBUG)
                .with_fields(field::mock("thing")),
        )
        .done()
        .run_with_handle();
    let subscriber = subscriber.with(filter);

    with_default(subscriber, || {
        tracing::trace!(disabled = true);
        tracing::info!("also disabled");
        tracing::info!(thing = 1);
        tracing::debug!(thing = 2);
        tracing::trace!(thing = 3);
    });

    finished.assert_finished();
}
