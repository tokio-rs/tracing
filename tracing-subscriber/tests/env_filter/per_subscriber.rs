//! Tests for using `EnvFilter` as a per-subscriber filter (rather than a global
//! `subscriber` filter).
#![cfg(feature = "registry")]
use super::*;
use tracing_mock::{span, subscriber};

#[test]
fn level_filter_event() {
    let filter: EnvFilter = "info".parse().expect("filter should parse");
    let (subscriber, handle) = subscriber::mock()
        .event(expect::event().at_level(Level::INFO))
        .event(expect::event().at_level(Level::WARN))
        .event(expect::event().at_level(Level::ERROR))
        .only()
        .run_with_handle();

    let _collect = tracing_subscriber::registry()
        .with(subscriber.with_filter(filter))
        .set_default();

    tracing::trace!("this should be disabled");
    tracing::info!("this shouldn't be");
    tracing::debug!(target: "foo", "this should also be disabled");
    tracing::warn!(target: "foo", "this should be enabled");
    tracing::error!("this should be enabled too");

    handle.assert_finished();
}

#[test]
fn same_name_spans() {
    let filter: EnvFilter = "[foo{bar}]=trace,[foo{baz}]=trace"
        .parse()
        .expect("filter should parse");
    let (subscriber, handle) = subscriber::mock()
        .new_span(
            expect::span()
                .named("foo")
                .at_level(Level::TRACE)
                .with_fields(expect::field("bar")),
        )
        .new_span(
            expect::span()
                .named("foo")
                .at_level(Level::TRACE)
                .with_fields(expect::field("baz")),
        )
        .only()
        .run_with_handle();

    let _collect = tracing_subscriber::registry()
        .with(subscriber.with_filter(filter))
        .set_default();

    tracing::trace_span!("foo", bar = 1);
    tracing::trace_span!("foo", baz = 1);

    handle.assert_finished();
}

#[test]
fn level_filter_event_with_target() {
    let filter: EnvFilter = "info,stuff=debug".parse().expect("filter should parse");
    let (subscriber, handle) = subscriber::mock()
        .event(expect::event().at_level(Level::INFO))
        .event(expect::event().at_level(Level::DEBUG).with_target("stuff"))
        .event(expect::event().at_level(Level::WARN).with_target("stuff"))
        .event(expect::event().at_level(Level::ERROR))
        .event(expect::event().at_level(Level::ERROR).with_target("stuff"))
        .only()
        .run_with_handle();

    let _collect = tracing_subscriber::registry()
        .with(subscriber.with_filter(filter))
        .set_default();

    tracing::trace!("this should be disabled");
    tracing::info!("this shouldn't be");
    tracing::debug!(target: "stuff", "this should be enabled");
    tracing::debug!("but this shouldn't");
    tracing::trace!(target: "stuff", "and neither should this");
    tracing::warn!(target: "stuff", "this should be enabled");
    tracing::error!("this should be enabled too");
    tracing::error!(target: "stuff", "this should be enabled also");

    handle.assert_finished();
}

#[test]
fn level_filter_event_with_target_and_span() {
    let filter: EnvFilter = "stuff[cool_span]=debug"
        .parse()
        .expect("filter should parse");

    let cool_span = span::named("cool_span");
    let (subscriber, handle) = subscriber::mock()
        .enter(cool_span.clone())
        .event(
            expect::event()
                .at_level(Level::DEBUG)
                .in_scope(vec![cool_span.clone()]),
        )
        .exit(cool_span)
        .only()
        .run_with_handle();

    let _collect = tracing_subscriber::registry()
        .with(subscriber.with_filter(filter))
        .set_default();

    {
        let _span = tracing::info_span!(target: "stuff", "cool_span").entered();
        tracing::debug!("this should be enabled");
    }

    tracing::debug!("should also be disabled");

    {
        let _span = tracing::info_span!("uncool_span").entered();
        tracing::debug!("this should be disabled");
    }

    handle.assert_finished();
}

#[test]
fn not_order_dependent() {
    // this test reproduces tokio-rs/tracing#623

    let filter: EnvFilter = "stuff=debug,info".parse().expect("filter should parse");
    let (subscriber, finished) = subscriber::mock()
        .event(expect::event().at_level(Level::INFO))
        .event(expect::event().at_level(Level::DEBUG).with_target("stuff"))
        .event(expect::event().at_level(Level::WARN).with_target("stuff"))
        .event(expect::event().at_level(Level::ERROR))
        .event(expect::event().at_level(Level::ERROR).with_target("stuff"))
        .only()
        .run_with_handle();

    let _collect = tracing_subscriber::registry()
        .with(subscriber.with_filter(filter))
        .set_default();

    tracing::trace!("this should be disabled");
    tracing::info!("this shouldn't be");
    tracing::debug!(target: "stuff", "this should be enabled");
    tracing::debug!("but this shouldn't");
    tracing::trace!(target: "stuff", "and neither should this");
    tracing::warn!(target: "stuff", "this should be enabled");
    tracing::error!("this should be enabled too");
    tracing::error!(target: "stuff", "this should be enabled also");

    finished.assert_finished();
}

#[test]
fn add_directive_enables_event() {
    // this test reproduces tokio-rs/tracing#591

    // by default, use info level
    let mut filter = EnvFilter::new(LevelFilter::INFO.to_string());

    // overwrite with a more specific directive
    filter = filter.add_directive("hello=trace".parse().expect("directive should parse"));

    let (subscriber, finished) = subscriber::mock()
        .event(expect::event().at_level(Level::INFO).with_target("hello"))
        .event(expect::event().at_level(Level::TRACE).with_target("hello"))
        .only()
        .run_with_handle();

    let _collect = tracing_subscriber::registry()
        .with(subscriber.with_filter(filter))
        .set_default();

    tracing::info!(target: "hello", "hello info");
    tracing::trace!(target: "hello", "hello trace");

    finished.assert_finished();
}

#[test]
fn span_name_filter_is_dynamic() {
    let filter: EnvFilter = "info,[cool_span]=debug"
        .parse()
        .expect("filter should parse");
    let cool_span = span::named("cool_span");
    let uncool_span = span::named("uncool_span");
    let (subscriber, finished) = subscriber::mock()
        .event(expect::event().at_level(Level::INFO))
        .enter(cool_span.clone())
        .event(
            expect::event()
                .at_level(Level::DEBUG)
                .in_scope(vec![cool_span.clone()]),
        )
        .enter(uncool_span.clone())
        .event(
            expect::event()
                .at_level(Level::WARN)
                .in_scope(vec![uncool_span.clone()]),
        )
        .event(
            expect::event()
                .at_level(Level::DEBUG)
                .in_scope(vec![uncool_span.clone()]),
        )
        .exit(uncool_span.clone())
        .exit(cool_span)
        .enter(uncool_span.clone())
        .event(
            expect::event()
                .at_level(Level::WARN)
                .in_scope(vec![uncool_span.clone()]),
        )
        .event(
            expect::event()
                .at_level(Level::ERROR)
                .in_scope(vec![uncool_span.clone()]),
        )
        .exit(uncool_span)
        .only()
        .run_with_handle();

    let _collect = tracing_subscriber::registry()
        .with(subscriber.with_filter(filter))
        .set_default();

    tracing::trace!("this should be disabled");
    tracing::info!("this shouldn't be");
    let cool_span = tracing::info_span!("cool_span");
    let uncool_span = tracing::info_span!("uncool_span");

    {
        let _enter = cool_span.enter();
        tracing::debug!("i'm a cool event");
        tracing::trace!("i'm cool, but not cool enough");
        let _enter2 = uncool_span.enter();
        tracing::warn!("warning: extremely cool!");
        tracing::debug!("i'm still cool");
    }

    {
        let _enter = uncool_span.enter();
        tracing::warn!("warning: not that cool");
        tracing::trace!("im not cool enough");
        tracing::error!("uncool error");
    }

    finished.assert_finished();
}

#[test]
fn multiple_dynamic_filters() {
    // Test that multiple dynamic (span) filters only apply to the subscribers
    // they're attached to.
    let (subscriber1, handle1) = {
        let span = span::named("span1");
        let filter: EnvFilter = "[span1]=debug".parse().expect("filter 1 should parse");
        let (subscriber, handle) = subscriber::named("subscriber1")
            .enter(span.clone())
            .event(
                expect::event()
                    .at_level(Level::DEBUG)
                    .in_scope(vec![span.clone()]),
            )
            .exit(span)
            .only()
            .run_with_handle();
        (subscriber.with_filter(filter), handle)
    };

    let (subscriber2, handle2) = {
        let span = span::named("span2");
        let filter: EnvFilter = "[span2]=info".parse().expect("filter 2 should parse");
        let (subscriber, handle) = subscriber::named("subscriber2")
            .enter(span.clone())
            .event(
                expect::event()
                    .at_level(Level::INFO)
                    .in_scope(vec![span.clone()]),
            )
            .exit(span)
            .only()
            .run_with_handle();
        (subscriber.with_filter(filter), handle)
    };

    let _collect = tracing_subscriber::registry()
        .with(subscriber1)
        .with(subscriber2)
        .set_default();

    tracing::info_span!("span1").in_scope(|| {
        tracing::debug!("hello from span 1");
        tracing::trace!("not enabled");
    });

    tracing::info_span!("span2").in_scope(|| {
        tracing::info!("hello from span 2");
        tracing::debug!("not enabled");
    });

    handle1.assert_finished();
    handle2.assert_finished();
}
