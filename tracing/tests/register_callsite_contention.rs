#![cfg(feature = "std")]

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, Barrier,
    },
    thread,
    time::Duration,
};
use tracing::{
    callsite::DefaultCallsite,
    metadata::{Kind, Metadata},
    span,
    subscriber::{Interest, Subscriber},
    Dispatch, Event, Level,
};

#[test]
fn contended_callsite_interest_is_conservative() {
    struct Blocking {
        calls: AtomicUsize,
        entered: mpsc::Sender<()>,
        resume: Arc<Barrier>,
    }

    impl Subscriber for Blocking {
        fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
            if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                self.entered.send(()).unwrap();
                self.resume.wait();
                return Interest::never();
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

    struct Always;

    impl Subscriber for Always {
        fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
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

    static __CALLSITE: DefaultCallsite = tracing::callsite2! {
        name: "contended",
        kind: Kind::EVENT,
        level: Level::INFO,
        fields:
    };

    let (tx, entered) = mpsc::channel();
    let resume = Arc::new(Barrier::new(2));
    let _blocking = Dispatch::new(Blocking {
        calls: AtomicUsize::new(0),
        entered: tx,
        resume: resume.clone(),
    });
    let th = thread::spawn(|| __CALLSITE.interest());

    entered.recv_timeout(Duration::from_secs(60)).unwrap();
    let _always = Dispatch::new(Always);
    resume.wait();
    let interest = th.join().unwrap();

    assert!(interest.is_sometimes());
    assert!(__CALLSITE.interest().is_sometimes());
}
