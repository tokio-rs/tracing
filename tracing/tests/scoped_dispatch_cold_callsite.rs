#![cfg(feature = "std")]

use tracing_mock::{expect, subscriber};

// Reproduces https://github.com/tokio-rs/tracing/issues/2874 (and #3611).
//
// While exactly one `Dispatch` is registered, `Rebuilder::JustOne` computes a
// cold callsite's interest from the *current thread's* default dispatcher
// instead of the registered one. If the callsite is first hit while that
// dispatch is not installed (here: same thread, before `with_default`; in
// #3611: another thread), `NoSubscriber` answers `Interest::never()`, which is
// cached forever, and the scoped subscriber never sees the callsite.
//
// This test needs its own binary: any other live `Dispatch` in the process
// clears `has_just_one`, takes the general path, and masks the bug.
#[test]
fn scoped_subscriber_observes_callsite_first_hit_before_it_was_set() {
    fn emit() {
        tracing::info!("shared callsite");
    }

    let (subscriber, handle) = subscriber::mock()
        .event(expect::event().with_fields(expect::msg("control")))
        .event(expect::event().with_fields(expect::msg("shared callsite")))
        .only()
        .run_with_handle();

    // Registers the dispatch (so it is the only one) without installing it.
    let dispatch = tracing::Dispatch::new(subscriber);

    // No default is set, so this is correctly dropped -- but it also registers
    // the callsite and caches its interest.
    emit();

    tracing::dispatcher::with_default(&dispatch, || {
        // A callsite first hit under the scoped dispatch works.
        tracing::info!("control");
        // The one first hit before it does not.
        emit();
    });

    handle.assert_finished();
}
