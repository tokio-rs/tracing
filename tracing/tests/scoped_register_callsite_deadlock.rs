#![cfg(feature = "std")]

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Mutex,
    },
    thread,
    time::Duration,
};
use tracing::{
    metadata::Metadata,
    span,
    subscriber::{self, Interest, Subscriber},
    Dispatch, Event,
};

#[test]
fn scoped_register_callsite_doesnt_deadlock() {
    struct TestSubscriber {
        entered: AtomicBool,
        lock: Mutex<()>,
    }

    impl Subscriber for TestSubscriber {
        fn register_callsite(&self, meta: &'static Metadata<'static>) -> Interest {
            if meta.target() == "outer" && !self.entered.swap(true, Ordering::SeqCst) {
                {
                    let _guard = self.lock.lock().unwrap();
                    tracing::info!(target: "nested", "registering");
                }
                let _dispatch = Dispatch::new(subscriber::NoSubscriber::default());
            } else if meta.target() == "nested" {
                let _guard = self.lock.lock().unwrap();
            }
            Interest::always()
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

    let (tx, didnt_hang) = mpsc::channel();
    let th = thread::spawn(move || {
        subscriber::with_default(
            TestSubscriber {
                entered: AtomicBool::new(false),
                lock: Mutex::new(()),
            },
            || tracing::info!(target: "outer", "hello world!"),
        );
        tx.send(()).unwrap();
    });

    didnt_hang
        .recv_timeout(Duration::from_secs(60))
        .expect("the thread must not have hung!");
    th.join().expect("thread should join successfully");
}
