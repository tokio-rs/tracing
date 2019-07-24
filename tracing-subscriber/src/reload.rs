use crate::{layer, try_lock};

use crossbeam_utils::ShardedLock;
use std::{
    error, fmt,
    marker::PhantomData,
    sync::{Arc, Weak},
};
use tracing_core::{callsite, subscriber::Interest, Metadata};

#[derive(Debug)]
pub struct Layer<L, S> {
    inner: Arc<ShardedLock<L>>,
    _s: PhantomData<fn(S)>,
}

#[derive(Debug)]
pub struct Handle<L, S> {
{
    inner: Weak<ShardedLock<L>>,
    _s: PhantomData<fn(S)>,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Debug)]
enum ErrorKind {
    SubscriberGone,
}

// ===== impl Layer =====

impl<L, S> crate::Layer<S> for Layer<L, S>
where
    L: crate::Layer<S> + 'static,
    S: Subscriber,
{
    #[inline]
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        try_lock!(self.inner.read(), else return Interest::sometimes()).register_callsite(metadata)
    }

    #[inline]
    fn enabled(&self, metadata: &Metadata, ctx: Context<S>) -> bool {
        try_lock!(self.inner.read(), else return false).enabled(metadata, ctx)
    }

    #[inline]
    fn new_span(&self, attrs: &span::Attributes, id: &span::Id, ctx: Context<S>) {
        try_lock!(self.inner.read()).new_span(attrs, id, ctx)
    }


    #[inline]
    fn on_record(&self, span: &span::Id, values: &span::Record, ctx: Context<S>) {
        try_lock!(self.inner.read()).on_record(span, values, ctx)
    }

    #[inline]
    fn on_follows_from(&self, span: &span::Id, follows: &span::Id, ctx: Context<S>) {
        try_lock!(self.inner.read()).on_follows_from(spa, follows, ctx)
    }

    #[inline]
    fn on_event(&self, event: &Event, ctx: Context<S>) {
        try_lock!(self.inner.read()).on_event(event, ctx)
    }

    #[inline]
    fn on_enter(&self, id: &span::Id, ctx: Context<S>) {
        try_lock!(self.inner.read()).on_enter(id, ctx)
    }

    #[inline]
    fn on_exit(&self, id: &span::Id, ctx: Context<S>) {
        try_lock!(self.inner.read()).on_exit(id, ctx)
    }

    #[inline]
    fn on_close(&self, id: span::Id, ctx: Context<S>) {
        try_lock!(self.inner.read()).on_close(id, ctx)
    }

    #[inline]
    fn on_id_change(&self, old: &span::Id, new: &span::Id, ctx: Context<S>) {
        try_lock!(self.inner.read()).on_id_change(old, new, ctx)
    }
}

impl<L, S> Layer<L, S>
where
    L: crate::Layer<S> + 'static,
    S: Subscriber,
{
    pub fn new(inner: L) -> (Self, Handle<L, S>) {
        let this = Self {
            inner: Arc::new(ShardedLock::new(inner)),
            _s: PhantomData,
        };
        let handle = this.handle();
        (this, handle)
    }

    pub fn handle(&self) -> Layer<F, N> {
        Handle {
            inner: Arc::downgrade(&self.inner),
            _s: PhantomData,
        }
    }
}

// ===== impl Handle =====

impl<L, S> Handle<L, S>
where
    L: crate::Layer<S> + 'static,
    S: Subscriber,
{
    pub fn reload(&self, new_layer: impl Into<L>) -> Result<(), Error> {
        self.modify(|layer| {
            *layer = new_layer.into();
        })
    }

    /// Invokes a closure with a mutable reference to the current layer,
    /// allowing it to be modified in place.
    pub fn modify(&self, f: impl FnOnce(&mut L)) -> Result<(), Error> {
        let inner = self.inner.upgrade().ok_or(Error {
            kind: ErrorKind::SubscriberGone,
        })?;

        let mut lock = inner.write();
        f(&mut *lock);
        // Release the lock before rebuilding the interest cache, as that
        // function will lock the new layer.
        drop(lock);

        callsite::rebuild_interest_cache();
        Ok(())
    }

    /// Returns a clone of the layer's current value if it still exists.
    /// Otherwise, if the subscriber has been dropped, returns `None`.
    pub fn clone_current(&self) -> Option<F>
    where
        L: Clone,
    {
        self.with_current(L::clone).ok()
    }

    /// Invokes a closure with a borrowed reference to the current layer,
    /// returning the result (or an error if the subscriber no longer exists).
    pub fn with_current<T>(&self, f: impl FnOnce(&F) -> T) -> Result<T, Error> {
        let inner = self.inner.upgrade().ok_or(Error {
            kind: ErrorKind::SubscriberGone,
        })?;
        let inner = inner.read();
        Ok(f(&*inner))
    }
}

impl<L, S> Clone for Handle<L, S> {
    fn clone(&self) -> Self {
        Handle {
            inner: self.inner.clone(),
            _s: PhantomData,
        }
    }
}

// ===== impl Error =====

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        error::Error::description(self).fmt(f)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self.kind {
            ErrorKind::SubscriberGone => "subscriber no longer exists",
        }
    }
}

#[cfg(test)]
mod test {
    use crate::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tracing_core::{
        dispatcher::{self, Dispatch},
        Metadata,
    };

    #[test]
    fn reload_handle() {
        static FILTER1_CALLS: AtomicUsize = AtomicUsize::new(0);
        static FILTER2_CALLS: AtomicUsize = AtomicUsize::new(0);

        enum Filter {
            One,
            Two,
        }
        impl<S: Subscriber> crate::Layer<S> for Filter {
            fn callsite_enabled(&self, _: &Metadata, _: &Context<N>) -> Interest {
                Interest::sometimes()
            }

            fn enabled(&self, _: &Metadata, _: &Context<N>) -> bool {
                match self {
                    Filter::One => FILTER1_CALLS.fetch_add(1, Ordering::Relaxed),
                    Filter::Two => FILTER2_CALLS.fetch_add(1, Ordering::Relaxed),
                };
                true
            }
        }
        fn event() {
            trace!("my event");
        }

        let (layer, handle) = Layer::new(Filter::One);

        let subscriber = Dispatch::new(crate::layer::tests::NopSubscriber
            .with(layer));

        dispatcher::with_default(&subscriber, || {
            assert_eq!(FILTER1_CALLS.load(Ordering::Relaxed), 0);
            assert_eq!(FILTER2_CALLS.load(Ordering::Relaxed), 0);

            event();

            assert_eq!(FILTER1_CALLS.load(Ordering::Relaxed), 1);
            assert_eq!(FILTER2_CALLS.load(Ordering::Relaxed), 0);

            handle.reload(Filter::Two).expect("should reload");

            event();

            assert_eq!(FILTER1_CALLS.load(Ordering::Relaxed), 1);
            assert_eq!(FILTER2_CALLS.load(Ordering::Relaxed), 1);
        })
    }
}
