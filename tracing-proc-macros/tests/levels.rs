mod support;
use support::*;

use tracing::subscriber::with_default;
use tracing::Level;
use tracing_proc_macros::trace;

#[test]
fn named_levels() {
    #[trace(level = "trace")]
    fn trace() {}

    #[trace(level = "Debug")]
    fn debug() {}

    #[trace(level = "INFO")]
    fn info() {}

    #[trace(level = "WARn")]
    fn warn() {}

    #[trace(level = "eRrOr")]
    fn error() {}
    let (subscriber, handle) = subscriber::mock()
        .new_span(span::mock().named("trace").at_level(Level::TRACE))
        .enter(span::mock().named("trace").at_level(Level::TRACE))
        .exit(span::mock().named("trace").at_level(Level::TRACE))
        .new_span(span::mock().named("debug").at_level(Level::DEBUG))
        .enter(span::mock().named("debug").at_level(Level::DEBUG))
        .exit(span::mock().named("debug").at_level(Level::DEBUG))
        .new_span(span::mock().named("info").at_level(Level::INFO))
        .enter(span::mock().named("info").at_level(Level::INFO))
        .exit(span::mock().named("info").at_level(Level::INFO))
        .new_span(span::mock().named("warn").at_level(Level::WARN))
        .enter(span::mock().named("warn").at_level(Level::WARN))
        .exit(span::mock().named("warn").at_level(Level::WARN))
        .new_span(span::mock().named("error").at_level(Level::ERROR))
        .enter(span::mock().named("error").at_level(Level::ERROR))
        .exit(span::mock().named("error").at_level(Level::ERROR))
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
    #[trace(level = 1)]
    fn trace() {}

    #[trace(level = 2)]
    fn debug() {}

    #[trace(level = 3)]
    fn info() {}

    #[trace(level = 4)]
    fn warn() {}

    #[trace(level = 5)]
    fn error() {}
    let (subscriber, handle) = subscriber::mock()
        .new_span(span::mock().named("trace").at_level(Level::TRACE))
        .enter(span::mock().named("trace").at_level(Level::TRACE))
        .exit(span::mock().named("trace").at_level(Level::TRACE))
        .new_span(span::mock().named("debug").at_level(Level::DEBUG))
        .enter(span::mock().named("debug").at_level(Level::DEBUG))
        .exit(span::mock().named("debug").at_level(Level::DEBUG))
        .new_span(span::mock().named("info").at_level(Level::INFO))
        .enter(span::mock().named("info").at_level(Level::INFO))
        .exit(span::mock().named("info").at_level(Level::INFO))
        .new_span(span::mock().named("warn").at_level(Level::WARN))
        .enter(span::mock().named("warn").at_level(Level::WARN))
        .exit(span::mock().named("warn").at_level(Level::WARN))
        .new_span(span::mock().named("error").at_level(Level::ERROR))
        .enter(span::mock().named("error").at_level(Level::ERROR))
        .exit(span::mock().named("error").at_level(Level::ERROR))
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
