mod support;
use self::support::*;
use tracing::Collect;
use tracing_subscriber::{filter, prelude::*, Subscribe};
use tracing::{level_filters::LevelFilter, Level};

// This test is just used to compare to the tests below
#[test]
fn just_subscriber() {
    let info = subscriber::named("info")
        .run()
        .with_filter(LevelFilter::INFO)
        .boxed();

    let collector = tracing_subscriber::registry().with(info);
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::INFO));
}

#[test]
fn subscriber_and_option_some_subscriber() {
    let info = subscriber::named("info")
        .run()
        .with_filter(LevelFilter::INFO)
        .boxed();

    let debug = subscriber::named("debug")
        .run()
        .with_filter(LevelFilter::DEBUG)
        .boxed();

    let collector = tracing_subscriber::registry().with(info).with(Some(debug));
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::DEBUG));
}

#[test]
fn subscriber_and_option_none_subscriber() {
    let error = subscriber::named("error")
        .run()
        .with_filter(LevelFilter::ERROR)
        .boxed();
    // None means the other layer takes control
    let collector = tracing_subscriber::registry()
        .with(error)
        .with(None::<ExpectSubscriber>);
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::ERROR));
}

#[test]
fn just_option_some_subscriber() {
    // Just a None means everything is off
    let collector = tracing_subscriber::registry().with(None::<ExpectSubscriber>);
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::OFF));
}

#[test]
fn just_option_none_subscriber() {
    let error = subscriber::named("error")
        .run()
        .with_filter(LevelFilter::ERROR)
        .boxed();
    let collector = tracing_subscriber::registry().with(Some(error));
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::ERROR));
}
