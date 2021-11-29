// Tests that depend on a count of the number of times their filter is evaluated
// cant exist in the same file with other tests that add subscribers to the
// registry. The registry was changed so that each time a new dispatcher is
// added all filters are re-evaluated. The tests being run only in separate
// threads with shared global state lets them interfere with each other

// liballoc is required because the test subscriber cannot be constructed
// statically
#![cfg(feature = "alloc")]

#[cfg(not(feature = "std"))]
extern crate std;
mod support;

use self::support::*;
use tracing::{span, Level};

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn filters_are_not_reevaluated_for_the_same_span() {
    // Asserts that the `span!` macro caches the result of calling
    // `Collector::enabled` for each span.
    let alice_count = Arc::new(AtomicUsize::new(0));
    let bob_count = Arc::new(AtomicUsize::new(0));
    let alice_count2 = alice_count.clone();
    let bob_count2 = bob_count.clone();

    let (collector, handle) = collector::mock()
        .with_filter(move |meta| match meta.name() {
            "alice" => {
                alice_count2.fetch_add(1, Ordering::Relaxed);
                false
            }
            "bob" => {
                bob_count2.fetch_add(1, Ordering::Relaxed);
                true
            }
            _ => false,
        })
        .run_with_handle();

    // Since this test is in its own file anyway, we can do this. Thus, this
    // test will work even with no-std.
    tracing::collect::set_global_default(collector).unwrap();

    // Enter "alice" and then "bob". The dispatcher expects to see "bob" but
    // not "alice."
    let alice = span!(Level::TRACE, "alice");
    let bob = alice.in_scope(|| {
        let bob = span!(Level::TRACE, "bob");
        bob.in_scope(|| ());
        bob
    });

    // The filter should have seen each span a single time.
    assert_eq!(alice_count.load(Ordering::Relaxed), 1);
    assert_eq!(bob_count.load(Ordering::Relaxed), 1);

    alice.in_scope(|| bob.in_scope(|| {}));

    // The subscriber should see "bob" again, but the filter should not have
    // been called.
    assert_eq!(alice_count.load(Ordering::Relaxed), 1);
    assert_eq!(bob_count.load(Ordering::Relaxed), 1);

    bob.in_scope(|| {});
    assert_eq!(alice_count.load(Ordering::Relaxed), 1);
    assert_eq!(bob_count.load(Ordering::Relaxed), 1);

    handle.assert_finished();
}
