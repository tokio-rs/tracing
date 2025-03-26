#![cfg(feature = "env-filter")]

mod per_subscriber;

use tracing::{self, collect::with_default, field, Level, Metadata, Value};
use tracing_mock::{
    collector::{self, MockCollector},
    expect,
};
use tracing_subscriber::{
    filter::{Directive, EnvFilter, LevelFilter, ValueMatch},
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

fn expect_span_and_event<F>(
    mock: MockCollector<F>,
    field: &str,
    value: &dyn Value,
) -> MockCollector<F>
where
    F: Fn(&Metadata<'_>) -> bool + 'static,
{
    let name = format!("enabled, with {field}");
    mock.new_span(
        expect::span()
            .named(&name)
            .at_level(Level::DEBUG)
            .with_fields(expect::field(field).with_value(value).only()),
    )
    .enter(expect::span().named(&name))
    .event(expect::event().at_level(Level::DEBUG))
    .exit(expect::span().named(name))
}

fn expect_span_and_no_event<F>(
    mock: MockCollector<F>,
    field: &str,
    value: &dyn Value,
) -> MockCollector<F>
where
    F: Fn(&Metadata<'_>) -> bool + 'static,
{
    let name = format!("disabled, with {field}");
    mock.new_span(
        expect::span()
            .named(&name)
            .at_level(Level::DEBUG)
            .with_fields(expect::field(field).with_value(value).only()),
    )
    .enter(expect::span().named(&name))
    .exit(expect::span().named(name))
}

#[test]
fn build_directives_can_match_on_primitive_like_string_and_other_unusual_values() {
    const PROBLEMATIC_VALUE: &str = "value,\"}]";
    let filter = EnvFilter::new("info")
        .add_directive(
            Directive::builder()
                .with_field("b", Some(ValueMatch::bool(false)))
                .with_level(LevelFilter::DEBUG)
                .build(),
        )
        .add_directive(
            Directive::builder()
                .with_field("sb", Some(ValueMatch::debug("true")))
                .with_level(LevelFilter::DEBUG)
                .build(),
        )
        .add_directive(
            Directive::builder()
                .with_field("n", Some(ValueMatch::i64(10)))
                .with_level(LevelFilter::DEBUG)
                .build(),
        )
        .add_directive(
            Directive::builder()
                .with_field("sn", Some(ValueMatch::debug("12")))
                .with_level(LevelFilter::DEBUG)
                .build(),
        )
        .add_directive(
            Directive::builder()
                .with_field("pv", Some(ValueMatch::debug(PROBLEMATIC_VALUE)))
                .with_level(LevelFilter::DEBUG)
                .build(),
        );

    let mut mock = collector::mock().event(expect::event().at_level(Level::INFO));
    mock = expect_span_and_event(mock, "b", &false);
    mock = expect_span_and_event(mock, "sb", &field::debug(true));
    mock = expect_span_and_event(mock, "n", &10i64);
    mock = expect_span_and_event(mock, "sn", &field::debug(12i64));
    mock = expect_span_and_event(mock, "pv", &field::display(PROBLEMATIC_VALUE));

    mock = expect_span_and_no_event(mock, "b", &true);
    mock = expect_span_and_no_event(mock, "sb", &field::debug(false));
    mock = expect_span_and_no_event(mock, "n", &11i64);
    mock = expect_span_and_no_event(mock, "sn", &field::debug(13i64));

    let (subscriber, finished) = mock.only().run_with_handle();
    let subscriber = subscriber.with(filter);

    with_default(subscriber, || {
        tracing::debug!("disabled, no value");
        tracing::info!("enabled, no value");

        {
            let _span = tracing::span!(Level::DEBUG, "enabled, with b", b = false).entered();
            tracing::debug!("enabled, with b");
        }
        {
            let _span = tracing::span!(Level::DEBUG, "enabled, with sb", sb = %true).entered();
            tracing::debug!("enabled, with sb");
        }
        {
            let _span = tracing::span!(Level::DEBUG, "enabled, with n", n = 10).entered();
            tracing::debug!("enabled, with n");
        }
        {
            let _span = tracing::span!(Level::DEBUG, "enabled, with sn",  sn = %12).entered();
            tracing::debug!("enabled, with sn");
        }
        {
            let _span = tracing::span!(Level::DEBUG, "enabled, with pv",  pv = %PROBLEMATIC_VALUE)
                .entered();
            tracing::debug!("enabled, with ,\"}}]field");
        }

        {
            let _span = tracing::span!(Level::DEBUG, "disabled, with b", b = true).entered();
            tracing::debug!("disabled, with b");
        }
        {
            let _span = tracing::span!(Level::DEBUG, "disabled, with sb", sb = %false).entered();
            tracing::debug!("disabled, with sb");
        }
        {
            let _span = tracing::span!(Level::DEBUG, "disabled, with n", n = 11).entered();
            tracing::debug!("disabled, with n");
        }
        {
            let _span = tracing::span!(Level::DEBUG, "disabled, with sn",  sn = %13).entered();
            tracing::debug!("disabled, with sn");
        }
    });

    finished.assert_finished();
}
