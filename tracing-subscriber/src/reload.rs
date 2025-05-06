//! Wrapper for a `Collect` or `Subscribe` to allow it to be dynamically reloaded.
//!
//! This module provides a type implementing [`Subscribe`] and [`Filter`]
//! which wraps another type implementing the corresponding trait. This
//! allows the wrapped type to be replaced with another
//! instance of that type at runtime.
//!
//! This can be used in cases where a subset of `Collect` or `Filter` functionality
//! should be dynamically reconfigured, such as when filtering directives may
//! change at runtime. Note that this subscriber introduces a (relatively small)
//! amount of overhead, and should thus only be used as needed.
//!
//! ## Note
//!
//! //! The [`Subscribe`] implementation is unable to implement downcasting functionality,
//! so certain `Subscribers` will fail to reload if wrapped in a `reload::Subscriber`.
//!
//! If you only want to be able to dynamically change the
//! `Filter` on your layer, prefer wrapping that `Filter` in the `reload::Subscriber`.
//!
//! [`Subscribe`]: crate::Subscribe
//! [`Filter`]: crate::subscribe::Filter
use crate::subscribe;
use crate::sync::RwLock;

use core::{any::TypeId, ptr::NonNull};
use std::{
    error, fmt,
    sync::{Arc, Weak},
};
use tracing_core::{
    callsite,
    collect::{Collect, Interest},
    span, Dispatch, Event, LevelFilter, Metadata,
};

/// Wraps a `Filter` or `Subscribe`, allowing it to be reloaded dynamically at runtime.
///
/// [`Filter`]: crate::subscribe::Filter
/// [`Subscribe`]: crate::Subscribe
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
    fn on_register_dispatch(&self, collector: &Dispatch) {
        try_lock!(self.inner.read()).on_register_dispatch(collector);
    }

    #[inline]
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        try_lock!(self.inner.read(), else return Interest::sometimes()).register_callsite(metadata)
    }

    #[inline]
    fn enabled(&self, metadata: &Metadata<'_>, ctx: &subscribe::Context<'_, C>) -> bool {
        try_lock!(self.inner.read(), else return false).enabled(metadata, ctx)
    }

    #[inline]
    fn on_new_span(
        &self,
        attrs: &span::Attributes<'_>,
        id: &span::Id,
        ctx: subscribe::Context<'_, C>,
    ) {
        try_lock!(self.inner.read()).on_new_span(attrs, id, ctx)
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
    fn event_enabled(&self, event: &Event<'_>, ctx: subscribe::Context<'_, C>) -> bool {
        try_lock!(self.inner.read(), else return false).event_enabled(event, ctx)
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

    #[inline]
    fn max_level_hint(&self) -> Option<LevelFilter> {
        try_lock!(self.inner.read(), else return None).max_level_hint()
    }

    #[doc(hidden)]
    unsafe fn downcast_raw(&self, id: TypeId) -> Option<NonNull<()>> {
        // Safety: it is generally unsafe to downcast through a reload, because
        // the pointer can be invalidated after the lock is dropped.
        // `NoneLayerMarker` is a special case because it
        // is never dereferenced.
        //
        // Additionally, even if the marker type *is* dereferenced (which it
        // never will be), the pointer should be valid even if the subscriber
        // is reloaded, because all `NoneLayerMarker` pointers that we return
        // actually point to the global static singleton `NoneLayerMarker`,
        // rather than to a field inside the lock.
        if id == TypeId::of::<subscribe::NoneLayerMarker>() {
            return try_lock!(self.inner.read(), else return None).downcast_raw(id);
        }

        None
    }
}

#[cfg(all(feature = "registry", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "registry", feature = "std"))))]
impl<S, C> crate::subscribe::Filter<C> for Subscriber<S>
where
    S: crate::subscribe::Filter<C> + 'static,
    C: Collect,
{
    #[inline]
    fn callsite_enabled(&self, metadata: &'static Metadata<'static>) -> Interest {
        try_lock!(self.inner.read(), else return Interest::sometimes()).callsite_enabled(metadata)
    }

    #[inline]
    fn enabled(&self, metadata: &Metadata<'_>, ctx: &subscribe::Context<'_, C>) -> bool {
        try_lock!(self.inner.read(), else return false).enabled(metadata, ctx)
    }

    #[inline]
    fn on_new_span(
        &self,
        attrs: &span::Attributes<'_>,
        id: &span::Id,
        ctx: subscribe::Context<'_, C>,
    ) {
        try_lock!(self.inner.read()).on_new_span(attrs, id, ctx)
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
    fn max_level_hint(&self) -> Option<LevelFilter> {
        try_lock!(self.inner.read(), else return None).max_level_hint()
    }
}

impl<T> Subscriber<T> {
    /// Wraps the given `Subscribe` or `Filter`,
    /// returning a subscriber or filter and a `Handle` that allows
    /// the inner type to be modified at runtime.
    ///
    /// [`Filter`]: crate::subscribe::Filter
    /// [`Subscribe`]: crate::Subscribe
    pub fn new(inner: T) -> (Self, Handle<T>) {
        let this = Self {
            inner: Arc::new(RwLock::new(inner)),
        };
        let handle = this.handle();
        (this, handle)
    }

    /// Returns a `Handle` that can be used to reload the wrapped `Subscribe`.
    pub fn handle(&self) -> Handle<T> {
        Handle {
            inner: Arc::downgrade(&self.inner),
        }
    }
}

// ===== impl Handle =====

impl<T> Handle<T> {
    /// Replace the current subscriber or filter with the provided `new_value`.
    ///
    /// [`Handle::reload`] cannot be used with the [`Filtered`](crate::filter::Filtered)
    /// subscriber; use [`Handle::modify`] instead (see [this issue] for additional details).
    ///
    /// However, if the _only_ the [`Filter`](crate::subscribe::Filter) needs to be modified,
    /// use `reload::Subscriber` to wrap the `Filter` directly.
    ///
    /// [this issue]: https://github.com/tokio-rs/tracing/issues/1629
    pub fn reload(&self, new_value: impl Into<T>) -> Result<(), Error> {
        self.modify(|object| {
            *object = new_value.into();
        })
    }

    /// Invokes a closure with a mutable reference to the current subscriber,
    /// allowing it to be modified in place.
    pub fn modify(&self, f: impl FnOnce(&mut T)) -> Result<(), Error> {
        let inner = self.inner.upgrade().ok_or(Error {
            kind: ErrorKind::CollectorGone,
        })?;

        let mut lock = try_lock!(inner.write(), else return Err(Error::poisoned()));
        f(&mut *lock);
        // Release the lock before rebuilding the interest cache, as that
        // function will lock the new subscriber.
        drop(lock);

        callsite::rebuild_interest_cache();

        // If the `log` crate compatibility feature is in use, set `log`'s max
        // level as well, in case the max `tracing` level changed. We do this
        // *after* rebuilding the interest cache, as that's when the `tracing`
        // max level filter is re-computed.
        #[cfg(feature = "tracing-log")]
        tracing_log::log::set_max_level(tracing_log::AsLog::as_log(
            &crate::filter::LevelFilter::current(),
        ));

        Ok(())
    }

    /// Returns a clone of the subscriber's current value if it still exists.
    /// Otherwise, if the collector has been dropped, returns `None`.
    pub fn clone_current(&self) -> Option<T>
    where
        T: Clone,
    {
        self.with_current(T::clone).ok()
    }

    /// Invokes a closure with a borrowed reference to the current subscriber,
    /// returning the result (or an error if the collector no longer exists).
    pub fn with_current<T2>(&self, f: impl FnOnce(&T) -> T2) -> Result<T2, Error> {
        let inner = self.inner.upgrade().ok_or(Error {
            kind: ErrorKind::CollectorGone,
        })?;
        let inner = try_lock!(inner.read(), else return Err(Error::poisoned()));
        Ok(f(&*inner))
    }
}

impl<T> Clone for Handle<T> {
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
