//! Wrapper for a `Collect` or `Subscribe` to allow it to be dynamically reloaded.
//!
//! This module provides a type implementing [`Subscribe`] which wraps another type implementing
//! the [`Subscribe`] trait, allowing the wrapped type to be replaced with another
//! instance of that type at runtime.
//!
//! This can be used in cases where a subset of `Collect` functionality
//! should be dynamically reconfigured, such as when filtering directives may
//! change at runtime. Note that this subscriber introduces a (relatively small)
//! amount of overhead, and should thus only be used as needed.
//!
//! [`Subscribe`]: super::subscriber::Subscribe
use crate::subscribe;
use crate::sync::RwLock;

use std::{
    error, fmt,
    sync::{Arc, Weak},
};
use tracing_core::{
    callsite,
    collect::{Collect, Interest},
    span, Event, Metadata,
};

/// Wraps a `Collect` or `Subscribe`, allowing it to be reloaded dynamically at runtime.
#[derive(Debug)]
pub struct Subscriber<S> {
    // TODO(eliza): this once used a `crossbeam_util::ShardedRwLock`. We may
    // eventually wish to replace it with a sharded lock implementation on top
    // of our internal `RwLock` wrapper type. If possible, we should profile
    // this first to determine if it's necessary.
    inner: Arc<RwLock<S>>,
}

/// Allows reloading the state of an associated `Collect`.
#[derive(Debug)]
pub struct Handle<S> {
    inner: Weak<RwLock<S>>,
}

/// Indicates that an error occurred when reloading a subscriber.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Debug)]
enum ErrorKind {
    CollectorGone,
    Poisoned,
}

// ===== impl Collect =====

impl<S, C> crate::Subscribe<C> for Subscriber<S>
where
    S: crate::Subscribe<C> + 'static,
    C: Collect,
{
    #[inline]
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        try_lock!(self.inner.read(), else return Interest::sometimes()).register_callsite(metadata)
    }

    #[inline]
    fn enabled(&self, metadata: &Metadata<'_>, ctx: subscribe::Context<'_, C>) -> bool {
        try_lock!(self.inner.read(), else return false).enabled(metadata, ctx)
    }

    #[inline]
    fn new_span(
        &self,
        attrs: &span::Attributes<'_>,
        id: &span::Id,
        ctx: subscribe::Context<'_, C>,
    ) {
        try_lock!(self.inner.read()).new_span(attrs, id, ctx)
    }

    #[inline]
    fn on_record(
        &self,
        span: &span::Id,
        values: &span::Record<'_>,
        ctx: subscribe::Context<'_, C>,
    ) {
        try_lock!(self.inner.read()).on_record(span, values, ctx)
    }

    #[inline]
    fn on_follows_from(&self, span: &span::Id, follows: &span::Id, ctx: subscribe::Context<'_, C>) {
        try_lock!(self.inner.read()).on_follows_from(span, follows, ctx)
    }

    #[inline]
    fn on_event(&self, event: &Event<'_>, ctx: subscribe::Context<'_, C>) {
        try_lock!(self.inner.read()).on_event(event, ctx)
    }

    #[inline]
    fn on_enter(&self, id: &span::Id, ctx: subscribe::Context<'_, C>) {
        try_lock!(self.inner.read()).on_enter(id, ctx)
    }

    #[inline]
    fn on_exit(&self, id: &span::Id, ctx: subscribe::Context<'_, C>) {
        try_lock!(self.inner.read()).on_exit(id, ctx)
    }

    #[inline]
    fn on_close(&self, id: span::Id, ctx: subscribe::Context<'_, C>) {
        try_lock!(self.inner.read()).on_close(id, ctx)
    }

    #[inline]
    fn on_id_change(&self, old: &span::Id, new: &span::Id, ctx: subscribe::Context<'_, C>) {
        try_lock!(self.inner.read()).on_id_change(old, new, ctx)
    }
}

impl<S> Subscriber<S> {
    /// Wraps the given `Subscribe`, returning a subscriber and a `Handle` that allows
    /// the inner type to be modified at runtime.
    pub fn new(inner: S) -> (Self, Handle<S>) {
        let this = Self {
            inner: Arc::new(RwLock::new(inner)),
        };
        let handle = this.handle();
        (this, handle)
    }

    /// Returns a `Handle` that can be used to reload the wrapped `Subscribe`.
    pub fn handle(&self) -> Handle<S> {
        Handle {
            inner: Arc::downgrade(&self.inner),
        }
    }
}

// ===== impl Handle =====

impl<S> Handle<S> {
    /// Replace the current subscriber with the provided `new_subscriber`.
    pub fn reload(&self, new_subscriber: impl Into<S>) -> Result<(), Error> {
        self.modify(|subscriber| {
            *subscriber = new_subscriber.into();
        })
    }

    /// Invokes a closure with a mutable reference to the current subscriber,
    /// allowing it to be modified in place.
    pub fn modify(&self, f: impl FnOnce(&mut S)) -> Result<(), Error> {
        let inner = self.inner.upgrade().ok_or(Error {
            kind: ErrorKind::CollectorGone,
        })?;

        let mut lock = try_lock!(inner.write(), else return Err(Error::poisoned()));
        f(&mut *lock);
        // Release the lock before rebuilding the interest cache, as that
        // function will lock the new subscriber.
        drop(lock);

        callsite::rebuild_interest_cache();
        Ok(())
    }

    /// Returns a clone of the subscriber's current value if it still exists.
    /// Otherwise, if the collector has been dropped, returns `None`.
    pub fn clone_current(&self) -> Option<S>
    where
        S: Clone,
    {
        self.with_current(S::clone).ok()
    }

    /// Invokes a closure with a borrowed reference to the current subscriber,
    /// returning the result (or an error if the collector no longer exists).
    pub fn with_current<T>(&self, f: impl FnOnce(&S) -> T) -> Result<T, Error> {
        let inner = self.inner.upgrade().ok_or(Error {
            kind: ErrorKind::CollectorGone,
        })?;
        let inner = try_lock!(inner.read(), else return Err(Error::poisoned()));
        Ok(f(&*inner))
    }
}

impl<S> Clone for Handle<S> {
    fn clone(&self) -> Self {
        Handle {
            inner: self.inner.clone(),
        }
    }
}

// ===== impl Error =====

impl Error {
    fn poisoned() -> Self {
        Self {
            kind: ErrorKind::Poisoned,
        }
    }

    /// Returns `true` if this error occurred because the subscriber was poisoned by
    /// a panic on another thread.
    pub fn is_poisoned(&self) -> bool {
        matches!(self.kind, ErrorKind::Poisoned)
    }

    /// Returns `true` if this error occurred because the `Collector`
    /// containing the reloadable subscriber was dropped.
    pub fn is_dropped(&self) -> bool {
        matches!(self.kind, ErrorKind::CollectorGone)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self.kind {
            ErrorKind::CollectorGone => "subscriber no longer exists",
            ErrorKind::Poisoned => "lock poisoned",
        };
        f.pad(msg)
    }
}

impl error::Error for Error {}
