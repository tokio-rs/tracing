use super::*;
use tracing::Collect;
use tracing_subscriber::{filter::Filtered, reload, subscribe::Filter};

fn mk_reload<S, F, C>(subscriber: S, filter: F) -> reload::Subscriber<Filtered<S, F, C>>
where
    S: Subscribe<C>,
    F: Filter<C>,
    C: Collect,
{
    let subscriber = subscriber.with_filter(filter);
    let (subscriber, _) = reload::Subscriber::new(subscriber);
    subscriber
}

#[test]
fn basic_subscriber_filters() {
    let (trace_subscriber, trace_handle) = subscriber::named("trace")
        .event(event::mock().at_level(Level::TRACE))
        .event(event::mock().at_level(Level::DEBUG))
        .event(event::mock().at_level(Level::INFO))
        .done()
        .run_with_handle();

    let (debug_subscriber, debug_handle) = subscriber::named("debug")
        .event(event::mock().at_level(Level::DEBUG))
        .event(event::mock().at_level(Level::INFO))
        .done()
        .run_with_handle();

    let (info_subscriber, info_handle) = subscriber::named("info")
        .event(event::mock().at_level(Level::INFO))
        .done()
        .run_with_handle();

    let trace_subscriber = mk_reload(trace_subscriber, LevelFilter::TRACE);
    let debug_subscriber = mk_reload(debug_subscriber, LevelFilter::DEBUG);
    let info_subscriber = mk_reload(info_subscriber, LevelFilter::INFO);

    let _subscriber = tracing_subscriber::registry()
        .with(trace_subscriber)
        .with(debug_subscriber)
        .with(info_subscriber)
        .set_default();

    tracing::trace!("hello trace");
    tracing::debug!("hello debug");
    tracing::info!("hello info");

    trace_handle.assert_finished();
    debug_handle.assert_finished();
    info_handle.assert_finished();
}

#[test]
fn basic_subscriber_filters_spans() {
    let (trace_subscriber, trace_handle) = subscriber::named("trace")
        .new_span(span::mock().at_level(Level::TRACE))
        .new_span(span::mock().at_level(Level::DEBUG))
        .new_span(span::mock().at_level(Level::INFO))
        .done()
        .run_with_handle();

    let (debug_subscriber, debug_handle) = subscriber::named("debug")
        .new_span(span::mock().at_level(Level::DEBUG))
        .new_span(span::mock().at_level(Level::INFO))
        .done()
        .run_with_handle();

    let (info_subscriber, info_handle) = subscriber::named("info")
        .new_span(span::mock().at_level(Level::INFO))
        .done()
        .run_with_handle();

    let trace_subscriber = mk_reload(trace_subscriber, LevelFilter::TRACE);
    let debug_subscriber = mk_reload(debug_subscriber, LevelFilter::DEBUG);
    let info_subscriber = mk_reload(info_subscriber, LevelFilter::INFO);

    let _subscriber = tracing_subscriber::registry()
        .with(trace_subscriber)
        .with(debug_subscriber)
        .with(info_subscriber)
        .set_default();

    tracing::trace_span!("hello trace");
    tracing::debug_span!("hello debug");
    tracing::info_span!("hello info");

    trace_handle.assert_finished();
    debug_handle.assert_finished();
    info_handle.assert_finished();
}

#[test]
fn global_filter_interests_are_cached() {
    let (expect, handle) = subscriber::mock()
        .event(event::mock().at_level(Level::WARN))
        .event(event::mock().at_level(Level::ERROR))
        .done()
        .run_with_handle();

    let filter = filter::filter_fn(|meta| {
        assert!(
            meta.level() <= &Level::INFO,
            "enabled should not be called for callsites disabled by the global filter"
        );
        meta.level() <= &Level::WARN
    });
    let expect = mk_reload(expect, filter);

    let _subscriber = tracing_subscriber::registry()
        .with(expect)
        .with(LevelFilter::INFO)
        .set_default();

    tracing::trace!("hello trace");
    tracing::debug!("hello debug");
    tracing::info!("hello info");
    tracing::warn!("hello warn");
    tracing::error!("hello error");

    handle.assert_finished();
}

#[test]
fn global_filters_affect_subscriber_filters() {
    let (expect, handle) = subscriber::named("debug")
        .event(event::mock().at_level(Level::INFO))
        .event(event::mock().at_level(Level::WARN))
        .event(event::mock().at_level(Level::ERROR))
        .done()
        .run_with_handle();
    let expect = mk_reload(expect, LevelFilter::DEBUG);

    let _subscriber = tracing_subscriber::registry()
        .with(expect)
        .with(LevelFilter::INFO)
        .set_default();

    tracing::trace!("hello trace");
    tracing::debug!("hello debug");
    tracing::info!("hello info");
    tracing::warn!("hello warn");
    tracing::error!("hello error");

    handle.assert_finished();
}

#[test]
fn filter_fn() {
    let (all, all_handle) = subscriber::named("all_targets")
        .event(event::msg("hello foo"))
        .event(event::msg("hello bar"))
        .done()
        .run_with_handle();

    let (foo, foo_handle) = subscriber::named("foo_target")
        .event(event::msg("hello foo"))
        .done()
        .run_with_handle();

    let (bar, bar_handle) = subscriber::named("bar_target")
        .event(event::msg("hello bar"))
        .done()
        .run_with_handle();

    let foo = {
        let filter = filter::filter_fn(|meta| meta.target().starts_with("foo"));
        mk_reload(foo, filter)
    };

    let bar = {
        let filter = filter::filter_fn(|meta| meta.target().starts_with("bar"));
        mk_reload(bar, filter)
    };

    let _subscriber = tracing_subscriber::registry()
        .with(all)
        .with(foo)
        .with(bar)
        .set_default();

    tracing::trace!(target: "foo", "hello foo");
    tracing::trace!(target: "bar", "hello bar");

    foo_handle.assert_finished();
    bar_handle.assert_finished();
    all_handle.assert_finished();
}
