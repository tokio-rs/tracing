use tracing::subscriber::with_default;
use tracing::Level;
use tracing_attributes::instrument;
use tracing_test::*;

#[test]
fn named_levels() {
    #[instrument(level = "trace")]
    fn trace() {}

    #[instrument(level = "Debug")]
    fn debug() {}

    #[instrument(level = "INFO")]
    fn info() {}

    #[instrument(level = "WARn")]
    fn warn() {}

    #[instrument(level = "eRrOr")]
    fn error() {}
    let (subscriber, handle) = subscriber::mock()
        .new_span(span("trace").at_level(Level::TRACE))
        .enter(span("trace").at_level(Level::TRACE))
        .exit(span("trace").at_level(Level::TRACE))
        .new_span(span("debug").at_level(Level::DEBUG))
        .enter(span("debug").at_level(Level::DEBUG))
        .exit(span("debug").at_level(Level::DEBUG))
        .new_span(span("info").at_level(Level::INFO))
        .enter(span("info").at_level(Level::INFO))
        .exit(span("info").at_level(Level::INFO))
        .new_span(span("warn").at_level(Level::WARN))
        .enter(span("warn").at_level(Level::WARN))
        .exit(span("warn").at_level(Level::WARN))
        .new_span(span("error").at_level(Level::ERROR))
        .enter(span("error").at_level(Level::ERROR))
        .exit(span("error").at_level(Level::ERROR))
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        trace();
        debug();
        info();
        warn();
        error();
    });

    handle.assert_finished();
}

#[test]
fn numeric_levels() {
    #[instrument(level = 1)]
    fn trace() {}

    #[instrument(level = 2)]
    fn debug() {}

    #[instrument(level = 3)]
    fn info() {}

    #[instrument(level = 4)]
    fn warn() {}

    #[instrument(level = 5)]
    fn error() {}
    let (subscriber, handle) = subscriber::mock()
        .new_span(span("trace").at_level(Level::TRACE))
        .enter(span("trace").at_level(Level::TRACE))
        .exit(span("trace").at_level(Level::TRACE))
        .new_span(span("debug").at_level(Level::DEBUG))
        .enter(span("debug").at_level(Level::DEBUG))
        .exit(span("debug").at_level(Level::DEBUG))
        .new_span(span("info").at_level(Level::INFO))
        .enter(span("info").at_level(Level::INFO))
        .exit(span("info").at_level(Level::INFO))
        .new_span(span("warn").at_level(Level::WARN))
        .enter(span("warn").at_level(Level::WARN))
        .exit(span("warn").at_level(Level::WARN))
        .new_span(span("error").at_level(Level::ERROR))
        .enter(span("error").at_level(Level::ERROR))
        .exit(span("error").at_level(Level::ERROR))
        .done()
        .run_with_handle();

    with_default(subscriber, || {
        trace();
        debug();
        info();
        warn();
        error();
    });

    handle.assert_finished();
}
