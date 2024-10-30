use super::*;
use tracing::Collect;
use tracing_subscriber::{
    filter::{self, LevelFilter},
    prelude::*,
    Subscribe,
};

fn filter_out_everything<S>() -> filter::DynFilterFn<S> {
    // Use dynamic filter fn to disable interest caching and max-level hints,
    // allowing us to put all of these tests in the same file.
    filter::dynamic_filter_fn(|_, _| false)
}

#[test]
fn option_some() {
    let (subscribe, handle) = subscriber::mock().only().run_with_handle();
    let subscribe = subscribe.with_filter(Some(filter_out_everything()));

    let _guard = tracing_subscriber::registry().with(subscribe).set_default();

    for i in 0..2 {
        tracing::info!(i);
    }

    handle.assert_finished();
}

#[test]
fn option_none() {
    let (subscribe, handle) = subscriber::mock()
        .event(expect::event())
        .event(expect::event())
        .only()
        .run_with_handle();
    let subscribe = subscribe.with_filter(None::<filter::DynFilterFn<_>>);

    let _guard = tracing_subscriber::registry().with(subscribe).set_default();

    for i in 0..2 {
        tracing::info!(i);
    }

    handle.assert_finished();
}

#[test]
fn option_mixed() {
    let (subscribe, handle) = subscriber::mock()
        .event(expect::event())
        .only()
        .run_with_handle();
    let subscribe = subscribe
        .with_filter(filter::dynamic_filter_fn(|meta, _ctx| {
            meta.target() == "interesting"
        }))
        .with_filter(None::<filter::DynFilterFn<_>>);

    let _guard = tracing_subscriber::registry().with(subscribe).set_default();

    tracing::info!(target: "interesting", x="foo");
    tracing::info!(target: "boring", x="bar");

    handle.assert_finished();
}

/// The lack of a max level hint from a `None` filter should result in no max
/// level hint when combined with other filters/subscribers.
#[test]
fn none_max_level_hint() {
    let (subscribe_none, handle_none) = subscriber::mock()
        .event(expect::event())
        .event(expect::event())
        .only()
        .run_with_handle();
    let subscribe_none = subscribe_none.with_filter(None::<filter::DynFilterFn<_>>);
    assert!(subscribe_none.max_level_hint().is_none());

    let (subscribe_filter_fn, handle_filter_fn) = subscriber::mock()
        .event(expect::event())
        .only()
        .run_with_handle();
    let max_level = Level::INFO;
    let subscribe_filter_fn = subscribe_filter_fn.with_filter(
        filter::dynamic_filter_fn(move |meta, _| return meta.level() <= &max_level)
            .with_max_level_hint(max_level),
    );
    assert_eq!(
        subscribe_filter_fn.max_level_hint(),
        Some(LevelFilter::INFO)
    );

    let subscriber = tracing_subscriber::registry()
        .with(subscribe_none)
        .with(subscribe_filter_fn);
    // The absence of a hint from the `None` filter upgrades the `INFO` hint
    // from the filter fn subscriber.
    assert!(subscriber.max_level_hint().is_none());

    let _guard = subscriber.set_default();
    tracing::info!(target: "interesting", x="foo");
    tracing::debug!(target: "sometimes_interesting", x="bar");

    handle_none.assert_finished();
    handle_filter_fn.assert_finished();
}

/// The max level hint from inside a `Some(filter)` filter should be propagated
/// and combined with other filters/subscribers.
#[test]
fn some_max_level_hint() {
    let (subscribe_some, handle_some) = subscriber::mock()
        .event(expect::event())
        .event(expect::event())
        .only()
        .run_with_handle();
    let subscribe_some = subscribe_some.with_filter(Some(
        filter::dynamic_filter_fn(move |meta, _| return meta.level() <= &Level::DEBUG)
            .with_max_level_hint(Level::DEBUG),
    ));
    assert_eq!(subscribe_some.max_level_hint(), Some(LevelFilter::DEBUG));

    let (subscribe_filter_fn, handle_filter_fn) = subscriber::mock()
        .event(expect::event())
        .only()
        .run_with_handle();
    let subscribe_filter_fn = subscribe_filter_fn.with_filter(
        filter::dynamic_filter_fn(move |meta, _| return meta.level() <= &Level::INFO)
            .with_max_level_hint(Level::INFO),
    );
    assert_eq!(
        subscribe_filter_fn.max_level_hint(),
        Some(LevelFilter::INFO)
    );

    let subscriber = tracing_subscriber::registry()
        .with(subscribe_some)
        .with(subscribe_filter_fn);
    // The `DEBUG` hint from the `Some` filter upgrades the `INFO` hint from the
    // filter fn subscriber.
    assert_eq!(subscriber.max_level_hint(), Some(LevelFilter::DEBUG));

    let _guard = subscriber.set_default();
    tracing::info!(target: "interesting", x="foo");
    tracing::debug!(target: "sometimes_interesting", x="bar");

    handle_some.assert_finished();
    handle_filter_fn.assert_finished();
}
