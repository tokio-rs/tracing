// A separate test crate for `Option<Filter>` for isolation from other tests
// that may influence the interest cache.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tracing_mock::{expect, layer};
use tracing_subscriber::{filter, prelude::*, Layer};

/// A `None` filter should always be interested in events, and it should not
/// needlessly degrade the caching of other filters.
#[test]
fn none_interest_cache() {
    let (layer_none, handle_none) = layer::mock()
        .event(expect::event())
        .event(expect::event())
        .only()
        .run_with_handle();
    let layer_none = layer_none.with_filter(None::<filter::DynFilterFn<_>>);

    let times_filtered = Arc::new(AtomicUsize::new(0));
    let (layer_filter_fn, handle_filter_fn) = layer::mock()
        .event(expect::event())
        .event(expect::event())
        .only()
        .run_with_handle();
    let layer_filter_fn = layer_filter_fn.with_filter(filter::filter_fn({
        let times_filtered = Arc::clone(&times_filtered);
        move |_| {
            times_filtered.fetch_add(1, Ordering::Relaxed);
            true
        }
    }));

    let subscriber = tracing_subscriber::registry()
        .with(layer_none)
        .with(layer_filter_fn);

    let _guard = subscriber.set_default();
    for _ in 0..2 {
        tracing::debug!(target: "always_interesting", x="bar");
    }

    // Per-layer filters cap callsite interest at `sometimes` (#2519, #3516), so
    // an accepted callsite is re-evaluated on every dispatch: once during
    // `register_callsite` plus once per event. The `None` filter must not
    // degrade caching any further than that (e.g. by forcing a callsite
    // re-registration).
    assert_eq!(times_filtered.load(Ordering::Relaxed), 3);
    handle_none.assert_finished();
    handle_filter_fn.assert_finished();
}
