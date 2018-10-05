use {span, subscriber::Subscriber, Event, SpanData, Meta};

use std::{
    cell::RefCell,
    collections::{
        HashSet,
        hash_map::DefaultHasher,
    },
    hash::{Hash, Hasher},
    fmt,
    sync::Arc,
};

thread_local! {
    static CURRENT_DISPATCH: RefCell<Current> = RefCell::new(Dispatch::none().with_invalidate());
}

struct Current {
    dispatch: Dispatch,
    seen: HashSet<u64>,
}

#[derive(Clone)]
pub struct Dispatch(Arc<dyn Subscriber + Send + Sync>);

impl Dispatch {
    pub fn none() -> Self {
        Dispatch(Arc::new(NoSubscriber))
    }

    pub fn current() -> Dispatch {
        CURRENT_DISPATCH.with(|current| {
            current.borrow().dispatch.clone()
        })
    }

    pub fn to<S>(subscriber: S) -> Self
    // TODO: Add some kind of `UnsyncDispatch`?
    where
        S: Subscriber + Send + Sync + 'static,
    {
        Dispatch(Arc::new(subscriber))
    }

    pub fn with<T>(&self, f: impl FnOnce() -> T) -> T {
        CURRENT_DISPATCH.with(|current| {
            let prior = current.replace(self.with_invalidate()).dispatch;
            let result = f();
            *current.borrow_mut() = prior.with_invalidate();
            result
        })
    }

    fn with_invalidate(&self) -> Current {
        Current {
            dispatch: self.clone(),
            seen: HashSet::new(),
        }
    }
}

impl fmt::Debug for Dispatch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("Dispatch(...)")
    }
}

impl Subscriber for Dispatch {
    fn should_invalidate_filter(&self, metadata: &Meta) -> bool {
        CURRENT_DISPATCH.with(|current| {
            let mut hasher = DefaultHasher::new();
            metadata.hash(&mut hasher);
            let hash = hasher.finish();

            if current.borrow().seen.contains(&hash) {
                self.0.should_invalidate_filter(metadata)
            } else {
                current.borrow_mut().seen.insert(hash);
                true
            }
        })

    }

    #[inline]
    fn enabled(&self, metadata: &Meta) -> bool {
        self.0.enabled(metadata)
    }

    #[inline]
    fn new_span(&self, new_span: &span::NewSpan) -> span::Id {
        self.0.new_span(new_span)
    }

    #[inline]
    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {
        self.0.observe_event(event)
    }

    #[inline]
    fn enter(&self, span: &SpanData) {
        self.0.enter(span)
    }

    #[inline]
    fn exit(&self, span: &SpanData) {
        self.0.exit(span)
    }
}

struct NoSubscriber;

impl Subscriber for NoSubscriber {
    fn enabled(&self, _metadata: &Meta) -> bool {
        false
    }

    fn new_span(&self, _new_span: &span::NewSpan) -> span::Id {
        span::Id::from_u64(0)
    }

    fn observe_event<'event, 'meta: 'event>(&self, _event: &'event Event<'event, 'meta>) {
        // Do nothing.
    }

    fn enter(&self, _span: &SpanData) {}

    fn exit(&self, _span: &SpanData) {}
}
