use {span, subscriber::Subscriber, Event, SpanData, Meta};

use std::{
    sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT},
    time::Instant,
};

static STATE: AtomicUsize = ATOMIC_USIZE_INIT;

// There are three different states that we care about: the logger's
// uninitialized, the logger's initializing (set_logger's been called but
// LOGGER hasn't actually been set yet), or the logger's active.
const UNINITIALIZED: usize = 0;
const INITIALIZING: usize = 1;
const INITIALIZED: usize = 2;
static mut DISPATCHER: &'static Subscriber = &NoDispatcher;

#[derive(Default)]
pub struct Builder {
    subscribers: Vec<Box<dyn Subscriber>>,
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_subscriber<T: Subscriber + 'static>(mut self, subscriber: T) -> Self {
        self.subscribers.push(Box::new(subscriber));
        self
    }

    pub fn try_init(self) -> Result<(), InitError> {
        unsafe {
            match STATE.compare_and_swap(UNINITIALIZED, INITIALIZING, Ordering::SeqCst) {
                UNINITIALIZED => {
                    DISPATCHER = &*Box::into_raw(Box::new(self));
                    STATE.store(INITIALIZED, Ordering::SeqCst);
                    Ok(())
                }
                INITIALIZING => {
                    while STATE.load(Ordering::SeqCst) == INITIALIZING {}
                    Err(InitError)
                }
                _ => Err(InitError),
            }
        }
    }

    pub fn init(self) {
        self.try_init().unwrap()
    }
}

impl Subscriber for Builder {
    fn enabled(&self, metadata: &Meta) -> bool {
        self.subscribers.iter()
            .any(|subscriber| subscriber.enabled(metadata))
    }

    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {
        for subscriber in &self.subscribers {
            subscriber.observe_event(event)
        }
    }

    fn new_span_id(&self, metadata: &Meta) -> span::Id {
        // TODO: this shouldn't let the first attached subscriber always be in
        // control of the span ID, but the dispatcher type is going away soon
        // anyway, so for now, it just needs to compile.
        self.subscribers.get(0)
            .map(|subscriber| subscriber.new_span_id(metadata))
            .unwrap_or_else(|| span::Id::from_u64(0))
    }

    #[inline]
    fn enter(&self, span: &SpanData, at: Instant) {
        for subscriber in &self.subscribers {
            subscriber.enter(span, at)
        }
    }

    #[inline]
    fn exit(&self, span: &SpanData, at: Instant) {
        for subscriber in &self.subscribers {
            subscriber.exit(span, at)
        }
    }
}

pub struct Dispatcher(&'static Subscriber);

impl Dispatcher {
    pub fn current() -> Dispatcher {
        Dispatcher(unsafe { DISPATCHER })
    }

    pub fn builder() -> Builder {
        Builder::new()
    }
}

impl Subscriber for Dispatcher {
    #[inline]
    fn enabled(&self, metadata: &Meta) -> bool {
        self.0.enabled(metadata)
    }

    #[inline]
    fn new_span_id(&self, metadata: &Meta) -> span::Id {
        self.0.new_span_id(metadata)
    }

    #[inline]
    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {
        self.0.observe_event(event)
    }

    #[inline]
    fn enter(&self, span: &SpanData, at: Instant) {
        self.0.enter(span, at)
    }

    #[inline]
    fn exit(&self, span: &SpanData, at: Instant) {
        self.0.exit(span, at)
    }
}

struct NoDispatcher;

#[derive(Debug)]
pub struct InitError;

impl Subscriber for NoDispatcher {
    fn enabled(&self, _metadata: &Meta) -> bool {
        false
    }

    fn new_span_id(&self, _metadata: &Meta) -> span::Id {
        span::Id::from_u64(0)
    }

    fn observe_event<'event, 'meta: 'event>(&self, _event: &'event Event<'event, 'meta>) {
        // Do nothing.
    }

    fn enter(&self, _span: &SpanData, _at: Instant) {}

    fn exit(&self, _span: &SpanData, _at: Instant) {}
}
