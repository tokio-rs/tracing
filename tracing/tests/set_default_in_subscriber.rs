// A subscriber that switches to another subscriber while handling an event,
// so that events emitted during handling go somewhere else. See #1655.
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tracing::{
    dispatcher::{self, Dispatch},
    span, Event, Metadata, Subscriber,
};

struct Counting(Arc<AtomicUsize>);

impl Subscriber for Counting {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }
    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
    fn event(&self, _: &Event<'_>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
    fn enter(&self, _: &span::Id) {}
    fn exit(&self, _: &span::Id) {}
}

struct Forwarding {
    events: Arc<AtomicUsize>,
    debug: Dispatch,
}

impl Subscriber for Forwarding {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }
    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
    fn event(&self, _: &Event<'_>) {
        self.events.fetch_add(1, Ordering::Relaxed);
        let _guard = dispatcher::set_default(&self.debug);
        tracing::debug!("handled an event");
    }
    fn enter(&self, _: &span::Id) {}
    fn exit(&self, _: &span::Id) {}
}

#[test]
fn set_default_while_handling_an_event() {
    let events = Arc::new(AtomicUsize::new(0));
    let debug_events = Arc::new(AtomicUsize::new(0));
    let subscriber = Forwarding {
        events: events.clone(),
        debug: Dispatch::new(Counting(debug_events.clone())),
    };

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("one");
        tracing::info!("two");
    });

    assert_eq!(events.load(Ordering::Relaxed), 2);
    assert_eq!(debug_events.load(Ordering::Relaxed), 2);
}
