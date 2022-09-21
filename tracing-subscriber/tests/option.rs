#![cfg(feature = "registry")]
use tracing_core::{collect::Interest, Collect, LevelFilter, Metadata};
use tracing_subscriber::{prelude::*, subscribe};

// A basic layer that returns its inner for `max_level_hint`
struct BasicLayer(Option<LevelFilter>);
impl<C: Collect> tracing_subscriber::Subscribe<C> for BasicLayer {
    fn register_callsite(&self, _m: &Metadata<'_>) -> Interest {
        Interest::sometimes()
    }

    fn enabled(&self, _m: &Metadata<'_>, _: subscribe::Context<'_, C>) -> bool {
        true
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        self.0
    }
}

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

/// Test that the `Layered` impl fallthrough the `max_level_hint` and `None` doesn't disable
/// everything, as well as ensuring the `Some` case also works.
#[test]
fn layered_fallthrough() {
    // None means the other layer takes control
    let subscriber = tracing_subscriber::registry()
        .with(BasicLayer(None))
        .with(None::<LevelFilter>);
    assert_eq!(subscriber.max_level_hint(), None);

    // The `None`-returning layer still wins
    let subscriber = tracing_subscriber::registry()
        .with(BasicLayer(None))
        .with(Some(LevelFilter::ERROR));
    assert_eq!(subscriber.max_level_hint(), None);

    // Check that we aren't doing anything truly wrong
    let subscriber = tracing_subscriber::registry()
        .with(BasicLayer(Some(LevelFilter::DEBUG)))
        .with(None::<LevelFilter>);
    assert_eq!(subscriber.max_level_hint(), Some(LevelFilter::DEBUG));
}
