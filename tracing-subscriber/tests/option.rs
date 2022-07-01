#![cfg(feature = "registry")]
use tracing::level_filters::LevelFilter;
use tracing::Collect;
use tracing_subscriber::prelude::*;

// This test is just used to compare to the tests below
#[test]
fn just_subscriber() {
    let collector = tracing_subscriber::registry().with(LevelFilter::INFO);
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::INFO));
}

#[test]
fn subscriber_and_option_some_subscriber() {
    let collector = tracing_subscriber::registry()
        .with(LevelFilter::INFO)
        .with(Some(LevelFilter::DEBUG));
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::DEBUG));
}

#[test]
fn subscriber_and_option_none_subscriber() {
    // None means the other layer takes control
    let collector = tracing_subscriber::registry()
        .with(LevelFilter::ERROR)
        .with(None::<LevelFilter>);
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::ERROR));
}

#[test]
fn just_option_some_subscriber() {
    // Just a None means everything is off
    let collector = tracing_subscriber::registry().with(None::<LevelFilter>);
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::OFF));
}

#[test]
fn just_option_none_subscriber() {
    let collector = tracing_subscriber::registry().with(Some(LevelFilter::ERROR));
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::ERROR));
}
