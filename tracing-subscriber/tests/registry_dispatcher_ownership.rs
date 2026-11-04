//! Span cleanup must notify its owning subscriber without changing the caller's dispatcher.

#![cfg(feature = "registry")]

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::dispatcher;
use tracing::span::Id;
use tracing::subscriber::NoSubscriber;
use tracing::Dispatch;
use tracing::Span;
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::layer::Identity;
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;

#[derive(Clone, Default)]
struct ClosedSpans(Arc<Mutex<Vec<&'static str>>>);

impl ClosedSpans {
    fn snapshot(&self) -> Vec<&'static str> {
        self.0.lock().unwrap().clone()
    }
}

impl<S> Layer<S> for ClosedSpans
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_close(&self, id: Id, context: Context<'_, S>) {
        let span = context.span(&id).expect("closing span is still available");
        self.0.lock().unwrap().push(span.metadata().name());
    }
}

fn last_child(closed: ClosedSpans) -> Span {
    let owner = Dispatch::new(tracing_subscriber::registry().with(closed));
    dispatcher::with_default(&owner, || {
        let parent = tracing::info_span!("parent");
        let child = tracing::info_span!(parent: &parent, "child");
        drop(parent);
        child
    })
}

fn assert_default_registry(expected: &Dispatch) {
    dispatcher::get_default(|actual| {
        assert!(std::ptr::eq(
            actual.downcast_ref::<Registry>().unwrap(),
            expected.downcast_ref::<Registry>().unwrap(),
        ));
    });
}

#[test]
fn final_child_close_uses_owning_dispatcher() {
    let closed = ClosedSpans::default();
    let child = last_child(closed.clone());
    std::thread::spawn(move || {
        let ambient = Dispatch::new(tracing_subscriber::registry());
        dispatcher::with_default(&ambient, || {
            drop(child);
            assert_default_registry(&ambient);
        });
    })
    .join()
    .expect("closing a span under another subscriber must not panic");

    assert_eq!(closed.snapshot(), vec!["child", "parent"]);
}

#[test]
fn queued_span_drop_keeps_each_owning_dispatcher() {
    let first = ClosedSpans::default();
    let second = ClosedSpans::default();
    let (sender, receiver) = std::sync::mpsc::channel();
    sender.send(last_child(first.clone())).unwrap();
    sender.send(last_child(second.clone())).unwrap();
    drop(sender);

    std::thread::spawn(move || {
        let ambient = Dispatch::new(NoSubscriber::default());
        dispatcher::with_default(&ambient, || {
            // Discard messages without ever entering their spans.
            drop(receiver);
            dispatcher::get_default(|current| assert!(current.is::<NoSubscriber>()));
        });
    })
    .join()
    .unwrap();

    assert_eq!(
        (first.snapshot(), second.snapshot()),
        (vec!["child", "parent"], vec!["child", "parent"]),
    );
}

#[test]
fn unwind_closes_parent_in_owning_dispatcher() {
    let closed = ClosedSpans::default();
    let child = last_child(closed.clone());
    std::thread::spawn(move || {
        let ambient = Dispatch::new(NoSubscriber::default());
        dispatcher::with_default(&ambient, || {
            let result: Result<(), _> = std::panic::catch_unwind(AssertUnwindSafe(move || {
                let _child = child;
                panic!("expected worker unwind");
            }));
            let payload = result.expect_err("the worker must unwind through span cleanup");
            assert_eq!(
                payload.downcast_ref::<&str>(),
                Some(&"expected worker unwind"),
            );
            dispatcher::get_default(|current| assert!(current.is::<NoSubscriber>()));
        });
    })
    .join()
    .unwrap();

    assert_eq!(closed.snapshot(), vec!["child", "parent"]);
}

#[test]
fn close_inside_get_default_preserves_reentrancy_and_ambient_dispatcher() {
    let closed = ClosedSpans::default();
    let mut child = Some(last_child(closed.clone()));
    let ambient = Dispatch::new(tracing_subscriber::registry());
    dispatcher::with_default(&ambient, || {
        dispatcher::get_default(|_| {
            dispatcher::get_default(|nested| assert!(nested.is::<NoSubscriber>()));
            drop(child.take());
            dispatcher::get_default(|nested| assert!(nested.is::<NoSubscriber>()));
        });
        assert_default_registry(&ambient);
    });

    assert_eq!(closed.snapshot(), vec!["child", "parent"]);
}

#[test]
fn deferred_span_data_clear_keeps_owning_dispatcher() {
    let closed = ClosedSpans::default();
    let child = last_child(closed.clone());
    let owner = child.with_subscriber(|(_, owner)| owner.clone()).unwrap();
    let id = child.id().unwrap();
    let data = owner
        .downcast_ref::<Registry>()
        .unwrap()
        .span_data(&id)
        .unwrap();
    let ambient = Dispatch::new(NoSubscriber::default());
    dispatcher::with_default(&ambient, || {
        drop(child);
        // A live lookup guard delays clearing the child slot and releasing its parent.
        assert_eq!(closed.snapshot(), vec!["child"]);
        drop(data);
        dispatcher::get_default(|current| assert!(current.is::<NoSubscriber>()));
    });

    assert_eq!(closed.snapshot(), vec!["child", "parent"]);
}

#[test]
fn bare_registry_span_drop_releases_owning_dispatcher() {
    let owner = Dispatch::new(tracing_subscriber::registry());
    let weak_owner = owner.downgrade();
    let child = dispatcher::with_default(&owner, || {
        let parent = tracing::info_span!("parent");
        let child = tracing::info_span!(parent: &parent, "child");
        drop(parent);
        child
    });
    let ambient = Dispatch::new(NoSubscriber::default());
    dispatcher::with_default(&ambient, || drop(child));
    drop(owner);

    assert!(
        weak_owner.upgrade().is_none(),
        "a registry without layers must not retain its own dispatcher",
    );
}

struct PanicOnClose;

impl<S: Subscriber> Layer<S> for PanicOnClose {
    fn on_close(&self, _id: Id, _context: Context<'_, S>) {
        panic!("expected inner close panic");
    }
}

#[test]
fn panicking_inner_close_callback_releases_owning_dispatcher() {
    let owner = Dispatch::new(
        tracing_subscriber::registry()
            .with(PanicOnClose)
            .with(Identity::new()),
    );
    let weak_owner = owner.downgrade();
    let child = dispatcher::with_default(&owner, || {
        let parent = tracing::info_span!("parent");
        let child = tracing::info_span!(parent: &parent, "child");
        drop(parent);
        child
    });
    let ambient = Dispatch::new(NoSubscriber::default());
    dispatcher::with_default(&ambient, || {
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| drop(child)));
        let payload = result.expect_err("the inner close callback must panic");
        assert_eq!(
            payload.downcast_ref::<&str>(),
            Some(&"expected inner close panic"),
        );
        dispatcher::get_default(|current| assert!(current.is::<NoSubscriber>()));
    });
    drop(owner);

    assert!(
        weak_owner.upgrade().is_none(),
        "unwinding through close callbacks must release the owning dispatcher",
    );
}
