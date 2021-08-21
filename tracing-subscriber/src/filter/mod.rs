//! [`Layer`]s that control which spans and events are enabled by the wrapped
//! subscriber.
//!
//! [`Layer`]: ../layer/trait.Layer.html
#[cfg(feature = "env-filter")]
mod env;
mod level;

pub use self::level::{LevelFilter, ParseError as LevelParseError};

#[cfg(feature = "env-filter")]
#[cfg_attr(docsrs, doc(cfg(feature = "env-filter")))]
pub use self::env::*;

use crate::{
    layer::{Context, Layer},
    registry,
};
use std::{fmt, num::NonZeroU8};
use tracing_core::{
    span,
    subscriber::{Interest, Subscriber},
    Event, Metadata,
};

/// A filter that determines whether a span or event is enabled.
pub trait Filter<S> {
    fn enabled(&self, meta: &Metadata<'_>, cx: &Context<'_, S>) -> bool;
    fn callsite_enabled(&self, meta: &'static Metadata<'static>, cx: &Context<'_, S>) -> Interest;
    fn max_level_hint(&self) -> Option<LevelFilter> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct Filtered<L, F> {
    filter: F,
    layer: L,
    id: FilterId,
}

#[derive(Copy, Clone, Debug)]
pub struct FilterId(NonZeroU8);

#[derive(Copy, Clone, Default)]
pub(crate) struct FilterMap {
    bits: usize,
}

// === impl Filtered ===

impl<S, L, F> Layer<S> for Filtered<L, F>
where
    S: Subscriber + for<'span> registry::LookupSpan<'span> + 'static,
    F: Filter<S> + 'static,
    L: Layer<S>,
{
    // TODO(eliza): can we figure out a nice way to make the `Filtered` layer
    // not call `is_enabled_for` in hooks that the inner layer doesn't actually
    // have real implementations of? probably not...
    //
    // it would be cool if there was some wild rust reflection way of checking
    // if a trait impl has the default impl of a trait method or not, but that's
    // almsot certainly impossible...right?

    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if self.enabled(metadata, Context::none()) {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn enabled(&self, metadata: &Metadata<'_>, cx: Context<'_, S>) -> bool {
        todo!()
    }

    fn new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, cx: Context<'_, S>) {
        if let Some(cx) = cx.if_enabled_for(id, self.id) {
            self.layer.new_span(attrs, id, cx)
        }
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
        // XXX(eliza) don't re-evaluate `enabled` here :(
        if self.filter.enabled(event.metadata(), &cx) {
            self.layer.on_event(event, cx.with_filter(self.id))
        }
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

// === impl FilterMap ===

impl FilterMap {
    pub(crate) fn set(&mut self, FilterId(idx): FilterId, enabled: bool) {
        let idx = idx.get() - 1;
        debug_assert!(idx < 64);
        if enabled {
            self.bits |= 1 << idx;
        } else {
            self.bits ^= 1 << idx;
        }
    }

    pub(crate) fn is_enabled(&self, FilterId(idx): FilterId) -> bool {
        let idx = idx.get() - 1;
        debug_assert!(idx < 64);
        self.bits & (1 << idx) != 0
    }
}

impl fmt::Debug for FilterMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilterMap")
            .field("bits", &format_args!("{:#b}", self.bits))
            .finish()
    }
}
