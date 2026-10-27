//! Regression test for <https://github.com/tokio-rs/tracing/issues/3644>.
//!
//! Dropping the last reference to a `Dispatch` while the interest cache is
//! being rebuilt must not run the subscriber's `Drop` while the dispatchers
//! lock is held. Otherwise a `Drop` impl that registers a callsite (for
//! example, by emitting an event) tries to take the same lock again and
//! deadlocks.
#![cfg(feature = "std")]

use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Mutex,
};
use std::thread;
use std::time::Duration;

use tracing_core::{
    callsite,
    dispatcher::Dispatch,
    metadata::{LevelFilter, Metadata},
    span, Event, Subscriber,
};

/// Holds the only strong reference to the first dispatcher.
static SLOT: Mutex<Option<Dispatch>> = Mutex::new(None);
/// Once set, the next interest rebuild releases the dispatcher in `SLOT`.
static ARMED: AtomicBool = AtomicBool::new(false);
static DROPPED: AtomicBool = AtomicBool::new(false);

/// Releases the last outside reference to its own `Dispatch` while the
/// interest cache is being rebuilt, and re-enters the callsite registry
/// when dropped.
struct DropsDuringRebuild;

impl Subscriber for DropsDuringRebuild {
    fn max_level_hint(&self) -> Option<LevelFilter> {
        if ARMED.swap(false, Ordering::SeqCst) {
            // Release the dispatcher's last outside reference while we are
            // being asked for our level hint, i.e. inside the rebuild.
            let dispatch = SLOT.lock().unwrap().take();
            drop(dispatch);
        }
        None
    }

    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }

    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}

    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}

    fn event(&self, _: &Event<'_>) {}

    fn enter(&self, _: &span::Id) {}

    fn exit(&self, _: &span::Id) {}
}

impl Drop for DropsDuringRebuild {
    fn drop(&mut self) {
        // Rebuilding the interest cache takes the dispatchers lock, like
        // registering a new callsite from an event emitted on drop does.
        callsite::rebuild_interest_cache();
        DROPPED.store(true, Ordering::SeqCst);
    }
}

struct Plain;

impl Subscriber for Plain {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }

    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}

    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}

    fn event(&self, _: &Event<'_>) {}

    fn enter(&self, _: &span::Id) {}

    fn exit(&self, _: &span::Id) {}
}

#[test]
fn dispatch_dropped_during_rebuild_does_not_deadlock() {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        *SLOT.lock().unwrap() = Some(Dispatch::new(DropsDuringRebuild));
        ARMED.store(true, Ordering::SeqCst);
        // Registering a second dispatcher rebuilds the interest cache over
        // both of them while holding the dispatchers lock.
        let _other = Dispatch::new(Plain);
        tx.send(()).unwrap();
    });

    rx.recv_timeout(Duration::from_secs(10))
        .expect("registering a dispatcher deadlocked while another one was dropped");
    assert!(SLOT.lock().unwrap().is_none());
    assert!(DROPPED.load(Ordering::SeqCst));
}
