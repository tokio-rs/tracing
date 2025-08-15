use tracing_core::{
    collect::{Collect, Interest},
    metadata::Metadata,
    span, Dispatch, Event, LevelFilter,
};

use crate::{
    filter,
    registry::LookupSpan,
    subscribe::{Context, Subscribe},
};
#[cfg(all(feature = "registry", feature = "std"))]
use crate::{filter::FilterId, registry::Registry};
use core::{
    any::{Any, TypeId},
    cmp, fmt,
    marker::PhantomData,
    ptr::NonNull,
};

/// A [collector] composed of a [collector] wrapped by one or more
/// [subscriber]s.
///
/// [subscriber]: crate::Subscribe
/// [collector]: tracing_core::Collect
#[derive(Clone)]
pub struct Layered<S, I, C = I> {
    /// The subscriber.
    subscriber: S,

    /// The inner value that `self.subscriber` was layered onto.
    ///
    /// If this is also a `Subscribe`, then this `Layered` will implement `Subscribe`.
    /// If this is a `Collect`, then this `Layered` will implement
    /// `Collect` instead.
    inner: I,

    // These booleans are used to determine how to combine `Interest`s and max
    // level hints when per-subscriber filters are in use.
    /// Is `self.inner` a `Registry`?
    ///
    /// If so, when combining `Interest`s, we want to "bubble up" its
    /// `Interest`.
    inner_is_registry: bool,

    /// Does `self.subscriber` have per-subscriber filters?
    ///
    /// This will be true if:
    /// - `self.inner` is a `Filtered`.
    /// - `self.inner` is a tree of `Layered`s where _all_ arms of those
    ///   `Layered`s have per-subscriber filters.
    ///
    /// Otherwise, if it's a `Layered` with one per-subscriber filter in one branch,
    /// but a non-per-subscriber-filtered subscriber in the other branch, this will be
    /// _false_, because the `Layered` is already handling the combining of
    /// per-subscriber filter `Interest`s and max level hints with its non-filtered
    /// `Subscribe`.
    has_subscriber_filter: bool,

    /// Does `self.inner` have per-subscriber filters?
    ///
    /// This is determined according to the same rules as
    /// `has_subscriber_filter` above.
    inner_has_subscriber_filter: bool,
    _s: PhantomData<fn(C)>,
}

// === impl Layered ===

impl<S, C> Layered<S, C>
where
    S: Subscribe<C>,
    C: Collect,
{
    /// Returns `true` if this `Collector` is the same type as `T`.
    pub fn is<T: Any>(&self) -> bool {
        self.downcast_ref::<T>().is_some()
    }

    /// Returns some reference to this `Collector` value if it is of type `T`,
    /// or `None` if it isn't.
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        unsafe {
            let raw = self.downcast_raw(TypeId::of::<T>())?;
            Some(&*(raw.cast().as_ptr()))
        }
    }
}

impl<S, C> Collect for Layered<S, C>
where
    S: Subscribe<C>,
    C: Collect,
{
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        self.pick_interest(self.subscriber.register_callsite(metadata), || {
            self.inner.register_callsite(metadata)
        })
    }

    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        if self.subscriber.enabled(metadata, self.ctx()) {
            // if the outer subscriber enables the callsite metadata, ask the collector.
            self.inner.enabled(metadata)
        } else {
            // otherwise, the callsite is disabled by the subscriber

            // If per-subscriber filters are in use, and we are short-circuiting
            // (rather than calling into the inner type), clear the current
            // per-subscriber filter `enabled` state.
            #[cfg(feature = "registry")]
            filter::FilterState::clear_enabled();

            false
        }
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        self.pick_level_hint(
            self.subscriber.max_level_hint(),
            self.inner.max_level_hint(),
            super::collector_is_none(&self.inner),
        )
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        let id = self.inner.new_span(span);
        self.subscriber.on_new_span(span, &id, self.ctx());
        id
    }

    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        self.inner.record(span, values);
        self.subscriber.on_record(span, values, self.ctx());
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        self.inner.record_follows_from(span, follows);
        self.subscriber.on_follows_from(span, follows, self.ctx());
    }

    fn event_enabled(&self, event: &Event<'_>) -> bool {
        if self.subscriber.event_enabled(event, self.ctx()) {
            // if the outer subscriber enables the event, ask the inner collector.
            self.inner.event_enabled(event)
        } else {
            // otherwise, the event is disabled by this subscriber
            false
        }
    }

    fn event(&self, event: &Event<'_>) {
        self.inner.event(event);
        self.subscriber.on_event(event, self.ctx());
    }

    fn enter(&self, span: &span::Id) {
        self.inner.enter(span);
        self.subscriber.on_enter(span, self.ctx());
    }

    fn exit(&self, span: &span::Id) {
        self.subscriber.on_exit(span, self.ctx());
        self.inner.exit(span);
    }

    fn clone_span(&self, old: &span::Id) -> span::Id {
        let new = self.inner.clone_span(old);
        if &new != old {
            self.subscriber.on_id_change(old, &new, self.ctx())
        };
        new
    }

    #[inline]
    fn drop_span(&self, id: span::Id) {
        self.try_close(id);
    }

    fn try_close(&self, id: span::Id) -> bool {
        #[cfg(all(feature = "registry", feature = "std"))]
        let subscriber = &self.inner as &dyn Collect;
        #[cfg(all(feature = "registry", feature = "std"))]
        let mut guard = subscriber
            .downcast_ref::<Registry>()
            .map(|registry| registry.start_close(id.clone()));
        if self.inner.try_close(id.clone()) {
            // If we have a registry's close guard, indicate that the span is
            // closing.
            #[cfg(all(feature = "registry", feature = "std"))]
            {
                if let Some(g) = guard.as_mut() {
                    g.set_closing()
                };
            }

            self.subscriber.on_close(id, self.ctx());
            true
        } else {
            false
        }
    }

    #[inline]
    fn current_span(&self) -> span::Current {
        self.inner.current_span()
    }

    #[doc(hidden)]
    unsafe fn downcast_raw(&self, id: TypeId) -> Option<NonNull<()>> {
        // Unlike the implementation of `Subscribe` for `Layered`, we don't have to
        // handle the "magic PSF downcast marker" here. If a `Layered`
        // implements `Collect`, we already know that the `inner` branch is
        // going to contain something that doesn't have per-subscriber filters (the
        // actual root `Collect`). Thus, a `Layered` that implements
        // `Collect` will always be propagating the root subscriber's
        // `Interest`/level hint, even if it includes a `Subscribe` that has
        // per-subscriber filters, because it will only ever contain subscribers where
        // _one_ child has per-subscriber filters.
        //
        // The complex per-subscriber filter detection logic is only relevant to
        // *trees* of subscribers, which involve the `Subscribe` implementation for
        // `Layered`, not *lists* of subscribers, where every `Layered` implements
        // `Collect`. Of course, a linked list can be thought of as a
        // degenerate tree...but luckily, we are able to make a type-level
        // distinction between individual `Layered`s that are definitely
        // list-shaped (their inner child implements `Collect`), and
        // `Layered`s that might be tree-shaped (the inner child is also a
        // `Subscribe`).

        // If downcasting to `Self`, return a pointer to `self`.
        if id == TypeId::of::<Self>() {
            return Some(NonNull::from(self).cast());
        }

        self.subscriber
            .downcast_raw(id)
            .or_else(|| self.inner.downcast_raw(id))
    }
}

impl<C, A, B> Subscribe<C> for Layered<A, B, C>
where
    A: Subscribe<C>,
    B: Subscribe<C>,
    C: Collect,
{
    fn on_register_dispatch(&self, collector: &Dispatch) {
        self.subscriber.on_register_dispatch(collector);
        self.inner.on_register_dispatch(collector);
    }

    fn on_subscribe(&mut self, collect: &mut C) {
        self.subscriber.on_subscribe(collect);
        self.inner.on_subscribe(collect);
    }

    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        self.pick_interest(self.subscriber.register_callsite(metadata), || {
            self.inner.register_callsite(metadata)
        })
    }

    fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, C>) -> bool {
        if self.subscriber.enabled(metadata, ctx.clone()) {
            // if the outer subscriber enables the callsite metadata, ask the inner subscriber.
            self.inner.enabled(metadata, ctx)
        } else {
            // otherwise, the callsite is disabled by this subscriber
            false
        }
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        self.pick_level_hint(
            self.subscriber.max_level_hint(),
            self.inner.max_level_hint(),
            super::subscriber_is_none(&self.inner),
        )
    }

    #[inline]
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, C>) {
        self.inner.on_new_span(attrs, id, ctx.clone());
        self.subscriber.on_new_span(attrs, id, ctx);
    }

    #[inline]
    fn on_record(&self, span: &span::Id, values: &span::Record<'_>, ctx: Context<'_, C>) {
        self.inner.on_record(span, values, ctx.clone());
        self.subscriber.on_record(span, values, ctx);
    }

    #[inline]
    fn on_follows_from(&self, span: &span::Id, follows: &span::Id, ctx: Context<'_, C>) {
        self.inner.on_follows_from(span, follows, ctx.clone());
        self.subscriber.on_follows_from(span, follows, ctx);
    }

    #[inline]
    fn event_enabled(&self, event: &Event<'_>, ctx: Context<'_, C>) -> bool {
        if self.subscriber.event_enabled(event, ctx.clone()) {
            // if the outer subscriber enables the event, ask the inner collector.
            self.inner.event_enabled(event, ctx)
        } else {
            // otherwise, the event is disabled by this subscriber
            false
        }
    }

    #[inline]
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, C>) {
        self.inner.on_event(event, ctx.clone());
        self.subscriber.on_event(event, ctx);
    }

    #[inline]
    fn on_enter(&self, id: &span::Id, ctx: Context<'_, C>) {
        self.inner.on_enter(id, ctx.clone());
        self.subscriber.on_enter(id, ctx);
    }

    #[inline]
    fn on_exit(&self, id: &span::Id, ctx: Context<'_, C>) {
        self.inner.on_exit(id, ctx.clone());
        self.subscriber.on_exit(id, ctx);
    }

    #[inline]
    fn on_close(&self, id: span::Id, ctx: Context<'_, C>) {
        self.inner.on_close(id.clone(), ctx.clone());
        self.subscriber.on_close(id, ctx);
    }

    #[inline]
    fn on_id_change(&self, old: &span::Id, new: &span::Id, ctx: Context<'_, C>) {
        self.inner.on_id_change(old, new, ctx.clone());
        self.subscriber.on_id_change(old, new, ctx);
    }

    #[doc(hidden)]
    unsafe fn downcast_raw(&self, id: TypeId) -> Option<NonNull<()>> {
        match id {
            // If downcasting to `Self`, return a pointer to `self`.
            id if id == TypeId::of::<Self>() => Some(NonNull::from(self).cast()),

            // Oh, we're looking for per-subscriber filters!
            //
            // This should only happen if we are inside of another `Layered`,
            // and it's trying to determine how it should combine `Interest`s
            // and max level hints.
            //
            // In that case, this `Layered` should be considered to be
            // "per-subscriber filtered" if *both* the outer subscriber and the inner
            // subscriber/subscriber have per-subscriber filters. Otherwise, this `Layered
            // should *not* be considered per-subscriber filtered (even if one or the
            // other has per subscriber filters). If only one `Subscribe` is per-subscriber
            // filtered, *this* `Layered` will handle aggregating the `Interest`
            // and level hints on behalf of its children, returning the
            // aggregate (which is the value from the &non-per-subscriber-filtered*
            // child).
            //
            // Yes, this rule *is* slightly counter-intuitive, but it's
            // necessary due to a weird edge case that can occur when two
            // `Layered`s where one side is per-subscriber filtered and the other
            // isn't are `Layered` together to form a tree. If we didn't have
            // this rule, we would actually end up *ignoring* `Interest`s from
            // the non-per-subscriber-filtered subscribers, since both branches would
            // claim to have PSF.
            //
            // If you don't understand this...that's fine, just don't mess with
            // it. :)
            id if filter::is_psf_downcast_marker(id) => self
                .subscriber
                .downcast_raw(id)
                .and(self.inner.downcast_raw(id)),

            // Otherwise, try to downcast both branches normally...
            _ => self
                .subscriber
                .downcast_raw(id)
                .or_else(|| self.inner.downcast_raw(id)),
        }
    }
}

impl<'a, S, C> LookupSpan<'a> for Layered<S, C>
where
    C: Collect + LookupSpan<'a>,
{
    type Data = C::Data;

    fn span_data(&'a self, id: &span::Id) -> Option<Self::Data> {
        self.inner.span_data(id)
    }

    #[cfg(all(feature = "registry", feature = "std"))]
    fn register_filter(&mut self) -> FilterId {
        self.inner.register_filter()
    }
}

impl<S, C> Layered<S, C>
where
    C: Collect,
{
    fn ctx(&self) -> Context<'_, C> {
        Context::new(&self.inner)
    }
}

impl<A, B, C> Layered<A, B, C>
where
    A: Subscribe<C>,
    C: Collect,
{
    pub(super) fn new(subscriber: A, inner: B, inner_has_subscriber_filter: bool) -> Self {
        #[cfg(all(feature = "registry", feature = "std"))]
        let inner_is_registry = TypeId::of::<C>() == TypeId::of::<crate::registry::Registry>();
        #[cfg(not(all(feature = "registry", feature = "std")))]
        let inner_is_registry = false;

        let inner_has_subscriber_filter = inner_has_subscriber_filter || inner_is_registry;
        let has_subscriber_filter = filter::subscriber_has_psf(&subscriber);
        Self {
            subscriber,
            inner,
            has_subscriber_filter,
            inner_has_subscriber_filter,
            inner_is_registry,
            _s: PhantomData,
        }
    }

    fn pick_interest(&self, outer: Interest, inner: impl FnOnce() -> Interest) -> Interest {
        if self.has_subscriber_filter {
            return inner();
        }

        // If the outer subscriber has disabled the callsite, return now so that
        // the inner subscriber/subscriber doesn't get its hopes up.
        if outer.is_never() {
            // If per-layer filters are in use, and we are short-circuiting
            // (rather than calling into the inner type), clear the current
            // per-layer filter interest state.
            #[cfg(feature = "registry")]
            filter::FilterState::take_interest();

            return outer;
        }

        // The `inner` closure will call `inner.register_callsite()`. We do this
        // before the `if` statement to  ensure that the inner subscriber is
        // informed that the callsite exists regardless of the outer subscriber's
        // filtering decision.
        let inner = inner();
        if outer.is_sometimes() {
            // if this interest is "sometimes", return "sometimes" to ensure that
            // filters are reevaluated.
            return outer;
        }

        // If there is a per-subscriber filter in the `inner` stack, and it returns
        // `never`, change the interest to `sometimes`, because the `outer`
        // subscriber didn't return `never`. This means that _some_ subscriber still wants
        // to see that callsite, even though the inner stack's per-subscriber filter
        // didn't want it. Therefore, returning `sometimes` will ensure
        // `enabled` is called so that the per-subscriber filter can skip that
        // span/event, while the `outer` subscriber still gets to see it.
        if inner.is_never() && self.inner_has_subscriber_filter {
            return Interest::sometimes();
        }

        // otherwise, allow the inner subscriber or collector to weigh in.
        inner
    }

    fn pick_level_hint(
        &self,
        outer_hint: Option<LevelFilter>,
        inner_hint: Option<LevelFilter>,
        inner_is_none: bool,
    ) -> Option<LevelFilter> {
        if self.inner_is_registry {
            return outer_hint;
        }

        if self.has_subscriber_filter && self.inner_has_subscriber_filter {
            return Some(cmp::max(outer_hint?, inner_hint?));
        }

        if self.has_subscriber_filter && inner_hint.is_none() {
            return None;
        }

        if self.inner_has_subscriber_filter && outer_hint.is_none() {
            return None;
        }

        // If the subscriber is `Option::None`, then we
        // want to short-circuit the layer underneath, if it
        // returns `None`, to override the `None` layer returning
        // `Some(OFF)`, which should ONLY apply when there are
        // no other layers that return `None`. Note this
        // `None` does not == `Some(TRACE)`, it means
        // something more like: "whatever all the other
        // layers agree on, default to `TRACE` if none
        // have an opinion". We also choose do this AFTER
        // we check for per-subscriber filters, which
        // have their own logic.
        //
        // Also note that this does come at some perf cost, but
        // this function is only called on initialization and
        // subscriber reloading.
        if super::subscriber_is_none(&self.subscriber) {
            return cmp::max(outer_hint, Some(inner_hint?));
        }

        // Similarly, if the layer on the inside is `None` and it returned an
        // `Off` hint, we want to override that with the outer hint.
        if inner_is_none && inner_hint == Some(LevelFilter::OFF) {
            return outer_hint;
        }

        cmp::max(outer_hint, inner_hint)
    }
}

impl<A, B, S> fmt::Debug for Layered<A, B, S>
where
    A: fmt::Debug,
    B: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(all(feature = "registry", feature = "std"))]
        let alt = f.alternate();
        let mut s = f.debug_struct("Layered");
        // These additional fields are more verbose and usually only necessary
        // for internal debugging purposes, so only print them if alternate mode
        // is enabled.
        #[cfg(all(feature = "registry", feature = "std"))]
        if alt {
            s.field("inner_is_registry", &self.inner_is_registry)
                .field("has_subscriber_filter", &self.has_subscriber_filter)
                .field(
                    "inner_has_subscriber_filter",
                    &self.inner_has_subscriber_filter,
                );
        }

        s.field("subscriber", &self.subscriber)
            .field("inner", &self.inner)
            .finish()
    }
}
