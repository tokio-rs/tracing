#![cfg(feature = "registry")]
#[path = "../support.rs"]
mod support;
use self::support::*;
mod filter_scopes;
mod targets;
mod trees;

use tracing::{level_filters::LevelFilter, Level};
use tracing_subscriber::{filter, prelude::*, Subscribe};

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

    let _subscriber = tracing_subscriber::registry()
        .with(trace_subscriber.with_filter(LevelFilter::TRACE))
        .with(debug_subscriber.with_filter(LevelFilter::DEBUG))
        .with(info_subscriber.with_filter(LevelFilter::INFO))
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

    let _subscriber = tracing_subscriber::registry()
        .with(trace_subscriber.with_filter(LevelFilter::TRACE))
        .with(debug_subscriber.with_filter(LevelFilter::DEBUG))
        .with(info_subscriber.with_filter(LevelFilter::INFO))
        .set_default();

    tracing::trace_span!("hello trace");
    tracing::debug_span!("hello debug");
    tracing::info_span!("hello info");

    trace_handle.assert_finished();
    debug_handle.assert_finished();
    info_handle.assert_finished();
}

#[test]
fn global_filters_subscribers_still_work() {
    let (expect, handle) = subscriber::mock()
        .event(event::mock().at_level(Level::INFO))
        .event(event::mock().at_level(Level::WARN))
        .event(event::mock().at_level(Level::ERROR))
        .done()
        .run_with_handle();

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
fn global_filter_interests_are_cached() {
    let (expect, handle) = subscriber::mock()
        .event(event::mock().at_level(Level::WARN))
        .event(event::mock().at_level(Level::ERROR))
        .done()
        .run_with_handle();

    let _subscriber = tracing_subscriber::registry()
        .with(expect.with_filter(filter::filter_fn(|meta| {
            assert!(
                meta.level() <= &Level::INFO,
                "enabled should not be called for callsites disabled by the global filter"
            );
            meta.level() <= &Level::WARN
        })))
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

    let _subscriber = tracing_subscriber::registry()
        .with(expect.with_filter(LevelFilter::DEBUG))
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

    let _subscriber = tracing_subscriber::registry()
        .with(all)
        .with(foo.with_filter(filter::filter_fn(|meta| meta.target().starts_with("foo"))))
        .with(bar.with_filter(filter::filter_fn(|meta| meta.target().starts_with("bar"))))
        .set_default();

    tracing::trace!(target: "foo", "hello foo");
    tracing::trace!(target: "bar", "hello bar");

    foo_handle.assert_finished();
    bar_handle.assert_finished();
    all_handle.assert_finished();
}

#[test]
fn vec_with_filters() {
    let (trace_subscriber, trace_handle) = subscriber::named("trace")
        .event(event::mock().at_level(Level::TRACE))
        .event(event::mock().at_level(Level::DEBUG))
        .event(event::mock().at_level(Level::INFO))
        .done()
        .run_with_handle();
    let trace_subscriber = trace_subscriber.with_filter(LevelFilter::TRACE);

    let (debug_subscriber, debug_handle) = subscriber::named("debug")
        .event(event::mock().at_level(Level::DEBUG))
        .event(event::mock().at_level(Level::INFO))
        .done()
        .run_with_handle();
    let debug_subscriber = debug_subscriber.with_filter(LevelFilter::DEBUG);

    let (info_subscriber, info_handle) = subscriber::named("info")
        .event(event::mock().at_level(Level::INFO))
        .done()
        .run_with_handle();
    let info_subscriber = info_subscriber.with_filter(LevelFilter::INFO);

    let _collector = tracing_subscriber::registry()
        .with(vec![trace_subscriber, debug_subscriber, info_subscriber])
        .set_default();

    tracing::trace!("hello trace");
    tracing::debug!("hello debug");
    tracing::info!("hello info");

    trace_handle.assert_finished();
    debug_handle.assert_finished();
    info_handle.assert_finished();
}

#[test]
fn vec_boxed() {
    let (unfiltered_subscriber, unfiltered_handle) = subscriber::named("unfiltered")
        .event(event::mock().at_level(Level::TRACE))
        .event(event::mock().at_level(Level::DEBUG))
        .event(event::mock().at_level(Level::INFO))
        .done()
        .run_with_handle();
    let unfiltered_subscriber = unfiltered_subscriber.boxed();

    let (debug_subscriber, debug_handle) = subscriber::named("debug")
        .event(event::mock().at_level(Level::DEBUG))
        .event(event::mock().at_level(Level::INFO))
        .done()
        .run_with_handle();
    let debug_subscriber = debug_subscriber.with_filter(LevelFilter::DEBUG).boxed();

    let (target_subscriber, target_handle) = subscriber::named("target")
        .event(event::mock().at_level(Level::INFO))
        .done()
        .run_with_handle();
    let target_subscriber = target_subscriber
        .with_filter(filter::filter_fn(|meta| meta.target() == "my_target"))
        .boxed();

    let _collector = tracing_subscriber::registry()
        .with(vec![
            unfiltered_subscriber,
            debug_subscriber,
            target_subscriber,
        ])
        .set_default();

    tracing::trace!("hello trace");
    tracing::debug!("hello debug");
    tracing::info!(target: "my_target", "hello my target");

    unfiltered_handle.assert_finished();
    debug_handle.assert_finished();
    target_handle.assert_finished();
}
