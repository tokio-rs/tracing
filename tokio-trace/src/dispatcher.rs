use {span, subscriber::Subscriber, Event, Meta};

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
    // TODO: agh, dispatchers really need not be sync, _only_ the exit part
    // really has to be Send + Sync. if this were in the subscriber crate, we
    // could slice and dice the sync requirement a little better...hmmm...
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
    fn new_span(&self, span: span::Data) -> span::Id {
        self.0.new_span(span)
    }

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
    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {
        self.0.observe_event(event)
    }

    #[inline]
    fn enter(&self, span: span::Id, state: span::State) {
        self.0.enter(span, state)
    }

    #[inline]
    fn exit(&self, span: span::Id, state: span::State) {
        self.0.exit(span, state)
    }
}

struct NoSubscriber;

impl Subscriber for NoSubscriber {
    fn new_span(&self, _span: span::Data) -> span::Id {
        span::Id::from_u64(0)
    }

    fn enabled(&self, _metadata: &Meta) -> bool {
        false
    }

    fn observe_event<'event, 'meta: 'event>(&self, _event: &'event Event<'event, 'meta>) {
        // Do nothing.
    }

    fn enter(&self, _span: span::Id, _state: span::State) {}

    fn exit(&self, _span: span::Id, _state: span::State) {}
}
