mod support;
use self::support::*;
use tracing::{self, subscriber::with_default, Level};
use tracing_subscriber::{filter::Filter, prelude::*};

#[test]
fn level_filter_event() {
    let filter: Filter = "info".parse().expect("filter should parse");
    let (subscriber, finished) = subscriber::mock()
        .event(event::mock().at_level(Level::INFO))
        .event(event::mock().at_level(Level::WARN))
        .event(event::mock().at_level(Level::ERROR))
        .done()
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
fn level_filter_event_with_target() {
    let filter: Filter = "info,stuff=debug".parse().expect("filter should parse");
    let (subscriber, finished) = subscriber::mock()
        .event(event::mock().at_level(Level::INFO))
        .event(event::mock().at_level(Level::DEBUG).with_target("stuff"))
        .event(event::mock().at_level(Level::WARN).with_target("stuff"))
        .event(event::mock().at_level(Level::ERROR))
        .event(event::mock().at_level(Level::ERROR).with_target("stuff"))
        .done()
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
fn span_name_filter_is_dynamic() {
    let filter: Filter = "info,[cool_span]=debug"
        .parse()
        .expect("filter should parse");
    let (subscriber, finished) = subscriber::mock()
        .event(event::mock().at_level(Level::INFO))
        .enter(span::mock().named("cool_span"))
        .event(event::mock().at_level(Level::DEBUG))
        .enter(span::mock().named("uncool_span"))
        .event(event::mock().at_level(Level::WARN))
        .event(event::mock().at_level(Level::DEBUG))
        .exit(span::mock().named("uncool_span"))
        .exit(span::mock().named("cool_span"))
        .enter(span::mock().named("uncool_span"))
        .event(event::mock().at_level(Level::WARN))
        .event(event::mock().at_level(Level::ERROR))
        .exit(span::mock().named("uncool_span"))
        .done()
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
fn field_filter_events() {
    let filter: Filter = "[{thing}]=debug".parse().expect("filter should parse");
    let (subscriber, finished) = subscriber::mock()
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

#[test]
fn field_filter_spans() {
    let filter: Filter = "[{enabled=true}]=debug"
        .parse()
        .expect("filter should parse");
    let (subscriber, finished) = subscriber::mock()
        .enter(span::mock().named("span1"))
        .event(
            event::mock()
                .at_level(Level::INFO)
                .with_fields(field::mock("thing")),
        )
        .exit(span::mock().named("span1"))
        .enter(span::mock().named("span2"))
        .exit(span::mock().named("span2"))
        .enter(span::mock().named("span3"))
        .event(
            event::mock()
                .at_level(Level::DEBUG)
                .with_fields(field::mock("thing")),
        )
        .exit(span::mock().named("span3"))
        .done()
        .run_with_handle();
    let subscriber = subscriber.with(filter);

    with_default(subscriber, || {
        tracing::trace!("disabled");
        tracing::info!("also disabled");
        tracing::info_span!("span1", enabled = true).in_scope(|| {
            tracing::info!(thing = 1);
        });
        tracing::debug_span!("span2", enabled = false, foo = "hi").in_scope(|| {
            tracing::warn!(thing = 2);
        });
        tracing::trace_span!("span3", enabled = true, answer = 42).in_scope(|| {
            tracing::debug!(thing = 2);
        });
    });

    finished.assert_finished();
}
