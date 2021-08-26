//! [`Layer`]s that control which spans and events are enabled by the wrapped
//! subscriber.
//!
//! [`Layer`]: ../layer/trait.Layer.html
#[cfg(feature = "env-filter")]
mod env;
mod level;

pub use self::level::{LevelFilter, ParseError as LevelParseError};
use std::{cell::Cell, thread_local};

#[cfg(feature = "env-filter")]
#[cfg_attr(docsrs, doc(cfg(feature = "env-filter")))]
pub use self::env::*;

use crate::{
    layer::{Context, Layer},
    registry,
};
use std::{fmt, marker::PhantomData};
use tracing_core::{
    span,
    subscriber::{Interest, Subscriber},
    Event, Metadata,
};

/// A filter that determines whether a span or event is enabled.
pub trait Filter<S> {
    fn enabled(&self, meta: &Metadata<'_>, cx: &Context<'_, S>) -> bool;

    fn callsite_enabled(&self, meta: &'static Metadata<'static>) -> Interest {
        let _ = meta;
        Interest::sometimes()
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct Filtered<L, F, S> {
    filter: F,
    layer: L,
    id: FilterId,
    _s: PhantomData<fn(S)>,
}

#[derive(Copy, Clone, Debug)]
pub struct FilterId(u8);

#[derive(Default, Copy, Clone)]
pub(crate) struct FilterMap {
    bits: u64,
}

thread_local! {
    pub(crate) static FILTERING: Cell<FilterMap> = Cell::new(FilterMap::default());
}

// === impl Filter ===

impl<S> Filter<S> for LevelFilter {
    fn enabled(&self, meta: &Metadata<'_>, _: &Context<'_, S>) -> bool {
        meta.level() <= self
    }

    fn callsite_enabled(&self, meta: &'static Metadata<'static>) -> Interest {
        if meta.level() <= self {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(*self)
    }
}

// === impl Filtered ===

impl<L, F, S> Filtered<L, F, S> {
    pub fn new(layer: L, filter: F) -> Self {
        Self {
            layer,
            filter,
            id: FilterId(255),
            _s: PhantomData,
        }
    }

    fn did_enable(&self, f: impl FnOnce()) {
        FILTERING.with(|filtering| {
            if filtering.get().is_enabled(self.id) {
                f();

                filtering.set(filtering.get().set(self.id, true));
            }
        })
    }
}

impl<S, L, F> Layer<S> for Filtered<L, F, S>
where
    S: Subscriber + for<'span> registry::LookupSpan<'span> + 'static,
    F: Filter<S> + 'static,
    L: Layer<S>,
{
    fn on_register(&mut self, subscriber: &mut S) {
        self.id = subscriber.register_filter();
        self.layer.on_register(subscriber);
    }

    // TODO(eliza): can we figure out a nice way to make the `Filtered` layer
    // not call `is_enabled_for` in hooks that the inner layer doesn't actually
    // have real implementations of? probably not...
    //
    // it would be cool if there was some wild rust reflection way of checking
    // if a trait impl has the default impl of a trait method or not, but that's
    // almsot certainly impossible...right?

    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        // self.filter.callsite_enabled(metadata)
        Interest::sometimes()
    }

    fn enabled(&self, metadata: &Metadata<'_>, cx: Context<'_, S>) -> bool {
        let enabled = self.filter.enabled(metadata, &cx.with_filter(self.id));
        FILTERING.with(|filtering| filtering.set(filtering.get().set(self.id, enabled)));
        true // keep filtering
    }

    fn new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, cx: Context<'_, S>) {
        self.did_enable(|| {
            self.layer.new_span(attrs, id, cx.with_filter(self.id));
        })
    }

    #[doc(hidden)]
    fn max_level_hint(&self) -> Option<LevelFilter> {
        self.filter.max_level_hint()
    }

    fn on_record(&self, span: &span::Id, values: &span::Record<'_>, cx: Context<'_, S>) {
        if let Some(cx) = cx.if_enabled_for(span, self.id) {
            self.layer.on_record(span, values, cx)
        }
    }

    fn on_follows_from(&self, span: &span::Id, follows: &span::Id, cx: Context<'_, S>) {
        // only call `on_follows_from` if both spans are enabled by us
        if cx.is_enabled_for(span, self.id) && cx.is_enabled_for(follows, self.id) {
            self.layer
                .on_follows_from(span, follows, cx.with_filter(self.id))
        }
    }

    fn on_event(&self, event: &Event<'_>, cx: Context<'_, S>) {
        self.did_enable(|| {
            self.layer.on_event(event, cx.with_filter(self.id));
        })
    }

    fn on_enter(&self, id: &span::Id, cx: Context<'_, S>) {
        if let Some(cx) = cx.if_enabled_for(id, self.id) {
            self.layer.on_enter(id, cx)
        }
    }

    fn on_exit(&self, id: &span::Id, cx: Context<'_, S>) {
        if let Some(cx) = cx.if_enabled_for(id, self.id) {
            self.layer.on_exit(id, cx)
        }
    }

    fn on_close(&self, id: span::Id, cx: Context<'_, S>) {
        if let Some(cx) = cx.if_enabled_for(&id, self.id) {
            self.layer.on_close(id, cx)
        }
    }

    // XXX(eliza): the existence of this method still makes me sad...
    fn on_id_change(&self, old: &span::Id, new: &span::Id, cx: Context<'_, S>) {
        if let Some(cx) = cx.if_enabled_for(old, self.id) {
            self.layer.on_id_change(old, new, cx)
        }
    }
}

// === impl FilterId ===

impl FilterId {
    pub(crate) fn new(id: u8) -> Self {
        assert!(id < 64, "filter IDs may not be greater than 64");
        Self(id)
    }
}

// === impl FilterMap ===

impl FilterMap {
    pub(crate) fn set(self, FilterId(idx): FilterId, enabled: bool) -> Self {
        debug_assert!(idx < 64 || idx == 255);
        if idx >= 64 {
            return self;
        }

        if enabled {
            Self {
                bits: self.bits & !(1 << idx),
            }
        } else {
            Self {
                bits: self.bits | (1 << idx),
            }
        }
    }

    pub(crate) fn is_enabled(self, FilterId(idx): FilterId) -> bool {
        debug_assert!(idx < 64 || idx == 255);
        if idx >= 64 {
            return false;
        }

        self.bits & (1 << idx) == 0
    }

    pub(crate) fn any_enabled(self) -> bool {
        self.bits != u64::MAX
    }
}

impl fmt::Debug for FilterMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilterMap")
            .field("bits", &format_args!("{:#b}", self.bits))
            .finish()
    }
}
