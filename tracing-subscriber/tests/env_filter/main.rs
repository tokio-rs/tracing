#![cfg(feature = "env-filter")]

mod per_subscriber;

use tracing::{self, collect::with_default, Level};
use tracing_mock::{collector, expect};
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    prelude::*,
};

#[test]
fn level_filter_event() {
    let filter: EnvFilter = "info".parse().expect("filter should parse");
    let (subscriber, finished) = collector::mock()
        .event(expect::event().at_level(Level::INFO))
        .event(expect::event().at_level(Level::WARN))
        .event(expect::event().at_level(Level::ERROR))
        .only()
        .run_with_handle();
    let subscriber = subscriber.with(filter);

    with_default(subscriber, || {
        tracing::trace!("this should be disabled");
        tracing::info!("this shouldn't be");
        tracing::debug!(target: "foo", "this should also be disabled");
        tracing::warn!(target: "foo", "this should be enabled");
        tracing::error!("this should be enabled too");
    });

    finished.assert_finished();
}

#[test]
fn same_name_spans() {
    let filter: EnvFilter = "[foo{bar}]=trace,[foo{baz}]=trace"
        .parse()
        .expect("filter should parse");
    let (subscriber, finished) = collector::mock()
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
    let subscriber = subscriber.with(filter);
    with_default(subscriber, || {
        tracing::trace_span!("foo", bar = 1);
        tracing::trace_span!("foo", baz = 1);
    });

    finished.assert_finished();
}

#[test]
fn level_filter_event_with_target() {
    let filter: EnvFilter = "info,stuff=debug".parse().expect("filter should parse");
    let (subscriber, finished) = collector::mock()
        .event(expect::event().at_level(Level::INFO))
        .event(expect::event().at_level(Level::DEBUG).with_target("stuff"))
        .event(expect::event().at_level(Level::WARN).with_target("stuff"))
        .event(expect::event().at_level(Level::ERROR))
        .event(expect::event().at_level(Level::ERROR).with_target("stuff"))
        .only()
        .run_with_handle();
    let subscriber = subscriber.with(filter);

    with_default(subscriber, || {
        tracing::trace!("this should be disabled");
        tracing::info!("this shouldn't be");
        tracing::debug!(target: "stuff", "this should be enabled");
        tracing::debug!("but this shouldn't");
        tracing::trace!(target: "stuff", "and neither should this");
        tracing::warn!(target: "stuff", "this should be enabled");
        tracing::error!("this should be enabled too");
        tracing::error!(target: "stuff", "this should be enabled also");
    });

    finished.assert_finished();
}

#[test]
fn not_order_dependent() {
    // this test reproduces tokio-rs/tracing#623

    let filter: EnvFilter = "stuff=debug,info".parse().expect("filter should parse");
    let (subscriber, finished) = collector::mock()
        .event(expect::event().at_level(Level::INFO))
        .event(expect::event().at_level(Level::DEBUG).with_target("stuff"))
        .event(expect::event().at_level(Level::WARN).with_target("stuff"))
        .event(expect::event().at_level(Level::ERROR))
        .event(expect::event().at_level(Level::ERROR).with_target("stuff"))
        .only()
        .run_with_handle();
    let subscriber = subscriber.with(filter);

    with_default(subscriber, || {
        tracing::trace!("this should be disabled");
        tracing::info!("this shouldn't be");
        tracing::debug!(target: "stuff", "this should be enabled");
        tracing::debug!("but this shouldn't");
        tracing::trace!(target: "stuff", "and neither should this");
        tracing::warn!(target: "stuff", "this should be enabled");
        tracing::error!("this should be enabled too");
        tracing::error!(target: "stuff", "this should be enabled also");
    });

    finished.assert_finished();
}

#[test]
fn add_directive_enables_event() {
    // this test reproduces tokio-rs/tracing#591

    // by default, use info level
    let mut filter = EnvFilter::new(LevelFilter::INFO.to_string());

    // overwrite with a more specific directive
    filter = filter.add_directive("hello=trace".parse().expect("directive should parse"));

    let (subscriber, finished) = collector::mock()
        .event(expect::event().at_level(Level::INFO).with_target("hello"))
        .event(expect::event().at_level(Level::TRACE).with_target("hello"))
        .only()
        .run_with_handle();
    let subscriber = subscriber.with(filter);

    with_default(subscriber, || {
        tracing::info!(target: "hello", "hello info");
        tracing::trace!(target: "hello", "hello trace");
    });

    finished.assert_finished();
}

#[test]
fn span_name_filter_is_dynamic() {
    let filter: EnvFilter = "info,[cool_span]=debug"
        .parse()
        .expect("filter should parse");
    let (subscriber, finished) = collector::mock()
        .event(expect::event().at_level(Level::INFO))
        .enter(expect::span().named("cool_span"))
        .event(expect::event().at_level(Level::DEBUG))
        .enter(expect::span().named("uncool_span"))
        .event(expect::event().at_level(Level::WARN))
        .event(expect::event().at_level(Level::DEBUG))
        .exit(expect::span().named("uncool_span"))
        .exit(expect::span().named("cool_span"))
        .enter(expect::span().named("uncool_span"))
        .event(expect::event().at_level(Level::WARN))
        .event(expect::event().at_level(Level::ERROR))
        .exit(expect::span().named("uncool_span"))
        .only()
        .run_with_handle();
    let subscriber = subscriber.with(filter);

    with_default(subscriber, || {
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

        let _enter = uncool_span.enter();
        tracing::warn!("warning: not that cool");
        tracing::trace!("im not cool enough");
        tracing::error!("uncool error");
    });

    finished.assert_finished();
}

#[test]
fn method_name_resolution() {
    #[allow(unused_imports)]
    use tracing_subscriber::subscribe::{Filter, Subscribe};

    let filter = EnvFilter::new("hello_world=info");
    filter.max_level_hint();
}

#[test]
fn parse_invalid_string() {
    assert!(EnvFilter::builder().parse(",!").is_err());
}

#[test]
fn parse_empty_string_no_default_directive() {
    let filter = EnvFilter::builder().parse("").expect("filter should parse");
    let (collector, finished) = collector::mock().only().run_with_handle();
    let subscriber = collector.with(filter);

    with_default(subscriber, || {
        tracing::trace!("this should be disabled");
        tracing::debug!("this should be disabled");
        tracing::info!("this should be disabled");
        tracing::warn!("this should be disabled");
        tracing::error!("this should be disabled");
    });

    finished.assert_finished();
}

#[test]
fn parse_empty_string_with_default_directive() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse("")
        .expect("filter should parse");
    let (collector, finished) = collector::mock()
        .event(expect::event().at_level(Level::INFO))
        .event(expect::event().at_level(Level::WARN))
        .event(expect::event().at_level(Level::ERROR))
        .only()
        .run_with_handle();
    let subscriber = collector.with(filter);

    with_default(subscriber, || {
        tracing::trace!("this should be disabled");
        tracing::debug!("this should be disabled");
        tracing::info!("this shouldn't be disabled");
        tracing::warn!("this shouldn't be disabled");
        tracing::error!("this shouldn't be disabled");
    });

    finished.assert_finished();
}

#[test]
fn new_invalid_string() {
    let filter = EnvFilter::new(",!");
    let (collector, finished) = collector::mock()
        .event(expect::event().at_level(Level::ERROR))
        .only()
        .run_with_handle();
    let subscriber = collector.with(filter);

    with_default(subscriber, || {
        tracing::trace!("this should be disabled");
        tracing::debug!("this should be disabled");
        tracing::info!("this should be disabled");
        tracing::warn!("this should be disabled");
        tracing::error!("this shouldn't be disabled");
    });

    finished.assert_finished();
}

#[test]
fn new_empty_string() {
    let filter = EnvFilter::new("");
    let (collector, finished) = collector::mock()
        .event(expect::event().at_level(Level::ERROR))
        .only()
        .run_with_handle();
    let subscriber = collector.with(filter);

    with_default(subscriber, || {
        tracing::trace!("this should be disabled");
        tracing::debug!("this should be disabled");
        tracing::info!("this should be disabled");
        tracing::warn!("this should be disabled");
        tracing::error!("this shouldn't be disabled");
    });

    finished.assert_finished();
}

#[test]
fn more_specific_static_filter_more_verbose() {
    let filter = EnvFilter::new("info,hello=debug");
    let (collector, finished) = collector::mock()
        .event(expect::event().at_level(Level::INFO))
        .event(expect::event().at_level(Level::DEBUG).with_target("hello"))
        .only()
        .run_with_handle();
    let subscriber = collector.with(filter);

    with_default(subscriber, || {
        tracing::info!("should be enabled");
        tracing::debug!("should be disabled");
        tracing::debug!(target: "hello", "should be enabled");
    });

    finished.assert_finished();
}

#[test]
fn more_specific_static_filter_less_verbose() {
    let filter = EnvFilter::new("info,hello=warn");
    let (collector, finished) = collector::mock()
        .event(expect::event().at_level(Level::INFO))
        .event(
            expect::event()
                .at_level(Level::WARN)
                .with_target("env_filter"),
        )
        .only()
        .run_with_handle();
    let subscriber = collector.with(filter);

    with_default(subscriber, || {
        tracing::info!("should be enabled");
        tracing::warn!("should be enabled");
        tracing::info!(target: "hello", "should be disabled");
    });

    finished.assert_finished();
}

#[test]
fn more_specific_dynamic_filter_more_verbose() {
    let filter = EnvFilter::new("info,[{hello=4}]=debug");
    let (collector, finished) = collector::mock()
        .new_span(expect::span().at_level(Level::INFO))
        .drop_span("enabled info")
        .new_span(
            expect::span()
                .at_level(Level::DEBUG)
                .with_fields(expect::field("hello").with_value(&4_u64)),
        )
        .drop_span("enabled debug")
        .event(expect::event().with_fields(expect::msg("marker")))
        .only()
        .run_with_handle();
    let subscriber = collector.with(filter);

    with_default(subscriber, || {
        tracing::info_span!("enabled info");
        tracing::debug_span!("disabled debug");
        tracing::debug_span!("enabled debug", hello = &4_u64);

        // .only() doesn't work when we don't enter/exit spans
        tracing::info!("marker");
    });

    finished.assert_finished();
}

/// This is a negative test. This functionality should work, but doesn't.
///
/// If an improvement to `EnvFilter` fixes this test, then the `#[should_panic]`
/// can be removed and the test kept as it is. If the test requires some sort of
/// modification, then care should be taken.
///
/// Fixing this test would resolve https://github.com/tokio-rs/tracing/issues/1388
/// (and probably a few more issues as well).
#[test]
#[should_panic(
    expected = "[more_specific_dynamic_filter_less_verbose] expected a new span \
    at level `Level(Warn)`,\n[more_specific_dynamic_filter_less_verbose] but \
    got one at level `Level(Info)` instead."
)]
fn more_specific_dynamic_filter_less_verbose() {
    let filter = EnvFilter::new("info,[{hello=4}]=warn");
    let (collector, finished) = collector::mock()
        .new_span(expect::span().at_level(Level::INFO))
        .drop_span("enabled info")
        .new_span(
            expect::span()
                .at_level(Level::WARN)
                .with_fields(expect::field("hello").with_value(&100_u64)),
        )
        .drop_span("enabled hello=100 warn")
        .new_span(
            expect::span()
                .at_level(Level::WARN)
                .with_fields(expect::field("hello").with_value(&4_u64)),
        )
        .drop_span("enabled hello=4 warn")
        .event(expect::event().with_fields(expect::msg("marker")))
        .only()
        .run_with_handle();
    let subscriber = collector.with(filter);

    with_default(subscriber, || {
        tracing::info_span!("enabled info");
        tracing::warn_span!("enabled hello=100 warn", hello = &100_u64);
        tracing::info_span!("disabled hello=4 info", hello = &4_u64);
        tracing::warn_span!("enabled hello=4 warn", hello = &4_u64);

        // .only() doesn't work when we don't enter/exit spans
        tracing::info!("marker");
    });

    finished.assert_finished();
}
