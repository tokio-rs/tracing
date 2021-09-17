//! Filter combinators
use crate::layer::{Context, Filter};
use std::cmp;
use tracing_core::{subscriber::Interest, LevelFilter, Metadata};

/// Combines two [`Filter`]s so that spans and events are enabled if and only if
/// _both_ filters return `true`.
#[derive(Debug, Clone)]
pub struct And<A, B>(A, B);

/// Combines two [`Filter`]s so that spans and events are enabled if *either* filter
/// returns `true`.
#[derive(Debug, Clone)]
pub struct Or<A, B>(A, B);

/// Inverts the result of a [`Filter`].
#[derive(Debug, Clone)]
pub struct Not<A>(A);

// === impl And ===

impl<A, B> And<A, B> {
    pub const fn new(a: A, b: B) -> Self {
        Self(a, b)
    }
}

impl<S, A, B> Filter<S> for And<A, B>
where
    A: Filter<S>,
    B: Filter<S>,
{
    #[inline]
    fn enabled(&self, meta: &Metadata<'_>, cx: &Context<'_, S>) -> bool {
        self.0.enabled(meta, cx) && self.1.enabled(meta, cx)
    }

    fn callsite_enabled(&self, meta: &'static Metadata<'static>) -> Interest {
        let a = self.0.callsite_enabled(meta);
        if a.is_never() {
            return a;
        }

        let b = self.1.callsite_enabled(meta);

        if !b.is_always() {
            return b;
        }

        a
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        // If either hint is `None`, return `None`. Otherwise, return the most restrictive.
        cmp::min(self.0.max_level_hint(), self.1.max_level_hint())
    }
}

// === impl Or ===

impl<A, B> Or<A, B> {
    pub const fn new(a: A, b: B) -> Self {
        Self(a, b)
    }
}

impl<S, A, B> Filter<S> for Or<A, B>
where
    A: Filter<S>,
    B: Filter<S>,
{
    #[inline]
    fn enabled(&self, meta: &Metadata<'_>, cx: &Context<'_, S>) -> bool {
        self.0.enabled(meta, cx) || self.1.enabled(meta, cx)
    }

    fn callsite_enabled(&self, meta: &'static Metadata<'static>) -> Interest {
        let a = self.0.callsite_enabled(meta);
        let b = self.1.callsite_enabled(meta);

        // If either filter will always enable the span or event, return `always`.
        if a.is_always() || b.is_always() {
            return Interest::always();
        }

        // Okay, if either filter will sometimes enable the span or event,
        // return `sometimes`.
        if a.is_sometimes() || b.is_sometimes() {
            return Interest::sometimes();
        }

        debug_assert!(
            a.is_never() && b.is_never(),
            "if neither filter was `always` or `sometimes`, both must be `never` (a={:?}; b={:?})",
            a,
            b,
        );
        Interest::never()
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        // If either hint is `None`, return `None`. Otherwise, return the less restrictive.
        Some(cmp::max(self.0.max_level_hint()?, self.1.max_level_hint()?))
    }
}

// === impl Not ===

impl<A> Not<A> {
    pub const fn new(a: A) -> Self {
        Self(a)
    }
}

impl<S, A> Filter<S> for Not<A>
where
    A: Filter<S>,
{
    #[inline]
    fn enabled(&self, meta: &Metadata<'_>, cx: &Context<'_, S>) -> bool {
        !self.0.enabled(meta, cx)
    }

    fn callsite_enabled(&self, meta: &'static Metadata<'static>) -> Interest {
        match self.0.callsite_enabled(meta) {
            i if i.is_always() => Interest::never(),
            i if i.is_never() => Interest::always(),
            _ => Interest::sometimes(),
        }
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        // TODO(eliza): figure this out???
        None
    }
}
