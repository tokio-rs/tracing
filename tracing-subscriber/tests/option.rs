#![cfg(feature = "registry")]
use tracing_core::{collect::Interest, Collect, LevelFilter, Metadata};
use tracing_subscriber::{prelude::*, subscribe};

// A basic layer that returns its inner for `max_level_hint`
#[derive(Debug)]
struct BasicLayer(Option<LevelFilter>);
impl<C: Collect> tracing_subscriber::Subscribe<C> for BasicLayer {
    fn register_callsite(&self, _m: &Metadata<'_>) -> Interest {
        Interest::sometimes()
    }

    fn enabled(&self, _m: &Metadata<'_>, _: &subscribe::Context<'_, C>) -> bool {
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

// Test that the `None` max level hint only applies if its the only layer
#[test]
fn none_outside_doesnt_override_max_level() {
    // None means the other layer takes control
    let subscriber = tracing_subscriber::registry()
        .with(BasicLayer(None))
        .with(None::<LevelFilter>);
    assert_eq!(
        subscriber.max_level_hint(),
        None,
        "\n stack: {:#?}",
        subscriber
    );

    // The `None`-returning layer still wins
    let subscriber = tracing_subscriber::registry()
        .with(BasicLayer(None))
        .with(Some(LevelFilter::ERROR));
    assert_eq!(
        subscriber.max_level_hint(),
        Some(LevelFilter::ERROR),
        "\n stack: {:#?}",
        subscriber
    );

    // Check that we aren't doing anything truly wrong
    let subscriber = tracing_subscriber::registry()
        .with(BasicLayer(Some(LevelFilter::DEBUG)))
        .with(None::<LevelFilter>);
    assert_eq!(
        subscriber.max_level_hint(),
        Some(LevelFilter::DEBUG),
        "\n stack: {:#?}",
        subscriber
    );

    // Test that per-subscriber filters aren't affected

    // One layer is None so it "wins"
    let subscriber = tracing_subscriber::registry()
        .with(BasicLayer(None))
        .with(None::<LevelFilter>.with_filter(LevelFilter::DEBUG));
    assert_eq!(
        subscriber.max_level_hint(),
        None,
        "\n stack: {:#?}",
        subscriber
    );

    // The max-levels wins
    let subscriber = tracing_subscriber::registry()
        .with(BasicLayer(Some(LevelFilter::INFO)))
        .with(None::<LevelFilter>.with_filter(LevelFilter::DEBUG));
    assert_eq!(
        subscriber.max_level_hint(),
        Some(LevelFilter::DEBUG),
        "\n stack: {:#?}",
        subscriber
    );

    // Test filter on the other layer
    let subscriber = tracing_subscriber::registry()
        .with(BasicLayer(Some(LevelFilter::INFO)).with_filter(LevelFilter::DEBUG))
        .with(None::<LevelFilter>);
    assert_eq!(
        subscriber.max_level_hint(),
        Some(LevelFilter::DEBUG),
        "\n stack: {:#?}",
        subscriber
    );
    let subscriber = tracing_subscriber::registry()
        .with(BasicLayer(None).with_filter(LevelFilter::DEBUG))
        .with(None::<LevelFilter>);
    assert_eq!(
        subscriber.max_level_hint(),
        Some(LevelFilter::DEBUG),
        "\n stack: {:#?}",
        subscriber
    );

    // The `OFF` from `None` over overridden.
    let subscriber = tracing_subscriber::registry()
        .with(BasicLayer(Some(LevelFilter::INFO)))
        .with(None::<LevelFilter>);
    assert_eq!(
        subscriber.max_level_hint(),
        Some(LevelFilter::INFO),
        "\n stack: {:#?}",
        subscriber
    );
}

// Test that the `None` max level hint only applies if its the only layer
#[test]
fn none_inside_doesnt_override_max_level() {
    // None means the other layer takes control
    let subscriber = tracing_subscriber::registry()
        .with(None::<LevelFilter>)
        .with(BasicLayer(None));
    assert_eq!(
        subscriber.max_level_hint(),
        None,
        "\n stack: {:#?}",
        subscriber
    );

    // The `None`-returning layer still wins
    let subscriber = tracing_subscriber::registry()
        .with(Some(LevelFilter::ERROR))
        .with(BasicLayer(None));
    assert_eq!(
        subscriber.max_level_hint(),
        Some(LevelFilter::ERROR),
        "\n stack: {:#?}",
        subscriber
    );

    // Check that we aren't doing anything truly wrong
    let subscriber = tracing_subscriber::registry()
        .with(None::<LevelFilter>)
        .with(BasicLayer(Some(LevelFilter::DEBUG)));
    assert_eq!(
        subscriber.max_level_hint(),
        Some(LevelFilter::DEBUG),
        "\n stack: {:#?}",
        subscriber
    );

    // Test that per-subscriber filters aren't affected

    // One layer is None so it "wins"
    let subscriber = tracing_subscriber::registry()
        .with(None::<LevelFilter>.with_filter(LevelFilter::DEBUG))
        .with(BasicLayer(None));
    assert_eq!(
        subscriber.max_level_hint(),
        None,
        "\n stack: {:#?}",
        subscriber
    );

    // The max-levels wins
    let subscriber = tracing_subscriber::registry()
        .with(None::<LevelFilter>.with_filter(LevelFilter::DEBUG))
        .with(BasicLayer(Some(LevelFilter::INFO)));
    assert_eq!(
        subscriber.max_level_hint(),
        Some(LevelFilter::DEBUG),
        "\n stack: {:#?}",
        subscriber
    );

    // Test filter on the other layer
    let subscriber = tracing_subscriber::registry()
        .with(None::<LevelFilter>)
        .with(BasicLayer(Some(LevelFilter::INFO)).with_filter(LevelFilter::DEBUG));
    assert_eq!(
        subscriber.max_level_hint(),
        Some(LevelFilter::DEBUG),
        "\n stack: {:#?}",
        subscriber
    );
    let subscriber = tracing_subscriber::registry()
        .with(None::<LevelFilter>)
        .with(BasicLayer(None).with_filter(LevelFilter::DEBUG));
    assert_eq!(
        subscriber.max_level_hint(),
        Some(LevelFilter::DEBUG),
        "\n stack: {:#?}",
        subscriber
    );

    // The `OFF` from `None` over overridden.
    let subscriber = tracing_subscriber::registry()
        .with(None::<LevelFilter>)
        .with(BasicLayer(Some(LevelFilter::INFO)));
    assert_eq!(
        subscriber.max_level_hint(),
        Some(LevelFilter::INFO),
        "\n stack: {:#?}",
        subscriber
    );
}

/// Tests that the logic tested in `doesnt_override_none` works through the reload subscriber
#[test]
fn reload_works_with_none() {
    let (subscriber1, handle1) = tracing_subscriber::reload::Subscriber::new(None::<BasicLayer>);
    let (subscriber2, _handle2) = tracing_subscriber::reload::Subscriber::new(None::<BasicLayer>);

    let subscriber = tracing_subscriber::registry()
        .with(subscriber1)
        .with(subscriber2);
    assert_eq!(subscriber.max_level_hint(), Some(LevelFilter::OFF));

    // reloading one should pass through correctly.
    handle1.reload(Some(BasicLayer(None))).unwrap();
    assert_eq!(subscriber.max_level_hint(), None);

    // Check pass-through of an actual level as well
    handle1
        .reload(Some(BasicLayer(Some(LevelFilter::DEBUG))))
        .unwrap();
    assert_eq!(subscriber.max_level_hint(), Some(LevelFilter::DEBUG));
}
