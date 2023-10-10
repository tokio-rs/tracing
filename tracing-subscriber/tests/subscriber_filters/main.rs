#![cfg(feature = "registry")]
mod filter_scopes;
mod option;
mod per_event;
mod targets;
mod trees;
mod vec;

use tracing::{level_filters::LevelFilter, Level};
use tracing_mock::{collector, event, expect, subscriber};
use tracing_subscriber::{filter, prelude::*, Subscribe};

#[test]
fn basic_subscriber_filters() {
    let (trace_subscriber, trace_handle) = subscriber::named("trace")
        .event(expect::event().at_level(Level::TRACE))
        .event(expect::event().at_level(Level::DEBUG))
        .event(expect::event().at_level(Level::INFO))
        .only()
        .run_with_handle();

    let (debug_subscriber, debug_handle) = subscriber::named("debug")
        .event(expect::event().at_level(Level::DEBUG))
        .event(expect::event().at_level(Level::INFO))
        .only()
        .run_with_handle();

    let (info_subscriber, info_handle) = subscriber::named("info")
        .event(expect::event().at_level(Level::INFO))
        .only()
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
        .new_span(expect::span().at_level(Level::TRACE))
        .new_span(expect::span().at_level(Level::DEBUG))
        .new_span(expect::span().at_level(Level::INFO))
        .only()
        .run_with_handle();

    let (debug_subscriber, debug_handle) = subscriber::named("debug")
        .new_span(expect::span().at_level(Level::DEBUG))
        .new_span(expect::span().at_level(Level::INFO))
        .only()
        .run_with_handle();

    let (info_subscriber, info_handle) = subscriber::named("info")
        .new_span(expect::span().at_level(Level::INFO))
        .only()
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
        .event(expect::event().at_level(Level::INFO))
        .event(expect::event().at_level(Level::WARN))
        .event(expect::event().at_level(Level::ERROR))
        .only()
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
        .event(expect::event().at_level(Level::WARN))
        .event(expect::event().at_level(Level::ERROR))
        .only()
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
        .event(expect::event().at_level(Level::INFO))
        .event(expect::event().at_level(Level::WARN))
        .event(expect::event().at_level(Level::ERROR))
        .only()
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
        .only()
        .run_with_handle();

    let (foo, foo_handle) = subscriber::named("foo_target")
        .event(event::msg("hello foo"))
        .only()
        .run_with_handle();

    let (bar, bar_handle) = subscriber::named("bar_target")
        .event(event::msg("hello bar"))
        .only()
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
