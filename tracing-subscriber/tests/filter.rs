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
        .event(event::mock().at_level(Level::WARN))
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
