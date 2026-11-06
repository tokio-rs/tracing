#![cfg(all(feature = "env-filter", feature = "registry"))]

use std::sync::{Arc, Mutex};
use tracing::{subscriber::with_default, Event, Level, Subscriber};
use tracing_subscriber::{filter::EnvFilter, layer::Context, prelude::*, Layer};

#[derive(Clone, Default)]
struct Events(Arc<Mutex<Vec<Level>>>);

impl<S: Subscriber> Layer<S> for Events {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        self.0.lock().unwrap().push(*event.metadata().level());
    }
}

fn with_filter(test: impl Fn(&Events)) {
    let filter = || EnvFilter::new("[debug_scope]=debug,[info_scope]=info");
    let events = Events::default();

    with_default(
        tracing_subscriber::registry()
            .with(events.clone())
            .with(filter()),
        || test(&events),
    );
    with_default(
        tracing_subscriber::registry().with(events.clone().with_filter(filter())),
        || test(&events),
    );
}

fn assert_levels(events: &Events, expected: &[Level]) {
    tracing::debug!("debug event");
    tracing::info!("info event");
    let actual = std::mem::take(&mut *events.0.lock().unwrap());
    assert_eq!(actual, expected);
}

#[test]
fn exiting_more_verbose_span_first_disables_its_level() {
    with_filter(|events| {
        let debug_span = tracing::info_span!("debug_scope");
        let info_span = tracing::info_span!("info_scope");
        let debug_guard = debug_span.enter();
        let info_guard = info_span.enter();
        assert_levels(events, &[Level::DEBUG, Level::INFO]);

        drop(debug_guard);
        assert_levels(events, &[Level::INFO]);

        drop(info_guard);
        assert_levels(events, &[]);
    });
}

#[test]
fn exiting_less_verbose_span_first_preserves_other_level() {
    with_filter(|events| {
        let info_span = tracing::info_span!("info_scope");
        let debug_span = tracing::info_span!("debug_scope");
        let info_guard = info_span.enter();
        let debug_guard = debug_span.enter();
        assert_levels(events, &[Level::DEBUG, Level::INFO]);

        drop(info_guard);
        assert_levels(events, &[Level::DEBUG, Level::INFO]);

        drop(debug_guard);
        assert_levels(events, &[]);
    });
}

#[test]
fn reentered_span_is_enabled_until_its_last_exit() {
    with_filter(|events| {
        let debug_span = tracing::info_span!("debug_scope");
        let info_span = tracing::info_span!("info_scope");
        let first_debug_guard = debug_span.enter();
        let second_debug_guard = debug_span.enter();
        let info_guard = info_span.enter();
        assert_levels(events, &[Level::DEBUG, Level::INFO]);

        drop(first_debug_guard);
        assert_levels(events, &[Level::DEBUG, Level::INFO]);

        drop(second_debug_guard);
        assert_levels(events, &[Level::INFO]);

        drop(info_guard);
        assert_levels(events, &[]);
    });
}
