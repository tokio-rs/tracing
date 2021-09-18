//! Filter combinators
use crate::layer::{Context, Filter};
use std::cmp;
use tracing_core::{subscriber::Interest, LevelFilter, Metadata};

/// Combines two [`Filter`]s so that spans and events are enabled if and only if
/// *both* filters return `true`.
///
/// Typically, this type is returned by the [`FilterExt::and`] method. See that
/// method's documentation for details.
///
/// [`Filter`]: crate::layer::Filter
/// [`FilterExt::and`]: crate::filter::FilterExt::and
#[derive(Debug, Clone)]
pub struct And<A, B>(A, B);

/// Combines two [`Filter`]s so that spans and events are enabled if *either* filter
/// returns `true`.
///
/// Typically, this type is returned by the [`FilterExt::or`] method. See that
/// method's documentation for details.
///
/// [`Filter`]: crate::layer::Filter
/// [`FilterExt::or`]: crate::filter::FilterExt::or
#[derive(Debug, Clone)]
pub struct Or<A, B>(A, B);

/// Inverts the result of a [`Filter`].
///
/// If the wrapped filter would enable a span or event, it will be disabled. If
/// it would disable a span or event, that span or event will be enabled.
///
/// Typically, this type is returned by the [`FilterExt::or`] method. See that
/// method's documentation for details.
///
/// [`Filter`]: crate::layer::Filter
/// [`FilterExt::or`]: crate::filter::FilterExt::or
#[derive(Debug, Clone)]
pub struct Not<A>(A);

// === impl And ===

impl<A, B> And<A, B> {
    /// Combines two [`Filter`]s so that spans and events are enabled if and only if
    /// *both* filters return `true`.
    ///
    /// # Examples
    ///
    /// Enabling spans or events if they have both a particular target *and* are
    /// above a certain level:
    ///
    /// ```
    /// use tracing_subscriber::{
    ///     filter::{filter_fn, LevelFilter, combinator::And};
    ///     prelude::*,
    /// };
    ///
    /// // Enables spans and events with targets starting with `interesting_target`:
    /// let target_filter = filter::filter_fn(|meta| {
    ///     meta.target().starts_with("interesting_target")
    /// });
    ///
    /// // Enables spans and events with levels `INFO` and below:
    /// let level_filter = LevelFilter::INFO;
    ///
    /// // Combine the two filters together so that a span or event is only enabled
    /// // if *both* filters would enable it:
    /// let filter = And::new(level_filter, target_filter);
    ///
    /// tracing_subscriber::registry()
    ///     .with(tracing_subscriber::fmt::layer().with_filter(filter))
    ///     .init();
    ///
    /// // This event will *not* be enabled:
    /// tracing::info!("an event with an uninteresting target");
    ///
    /// // This event *will* be enabled:
    /// tracing::info!(target: "interesting_target", "a very interesting event");
    ///
    /// // This event will *not* be enabled:
    /// tracing::debug!(target: "interesting_target", "interesting debug event...");
    /// ```
    ///
    /// [`Filter`]: crate::layer::Filter
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
    /// Combines two [`Filter`]s so that spans and events are enabled if *either* filter
    /// returns `true`.
    ///
    /// # Examples
    ///
    /// Enabling spans and events at the `INFO` level and above, and all spans
    /// and events with a particular target:
    ///
    /// ```
    /// use tracing_subscriber::{
    ///     filter::{filter_fn, LevelFilter, combinator::Or};
    ///     prelude::*,
    /// };
    ///
    /// // Enables spans and events with targets starting with `interesting_target`:
    /// let target_filter = filter::filter_fn(|meta| {
    ///     meta.target().starts_with("interesting_target")
    /// });
    ///
    /// // Enables spans and events with levels `INFO` and below:
    /// let level_filter = LevelFilter::INFO;
    ///
    /// // Combine the two filters together so that a span or event is enabled
    /// // if it is at INFO or lower, or if it has a target starting with
    /// // `interesting_target`.
    /// let filter = Or::new(level_filter, target_filter);
    ///
    /// tracing_subscriber::registry()
    ///     .with(tracing_subscriber::fmt::layer().with_filter(filter))
    ///     .init();
    ///
    /// // This event will *not* be enabled:
    /// tracing::debug!("an uninteresting event");
    ///
    /// // This event *will* be enabled:
    /// tracing::info!("an uninteresting INFO event");
    ///
    /// // This event *will* be enabled:
    /// tracing::info!(target: "interesting_target", "a very interesting event");
    ///
    /// // This event *will* be enabled:
    /// tracing::debug!(target: "interesting_target", "interesting debug event...");
    /// ```
    ///
    /// Enabling a higher level for a particular target by using `Or` in
    /// conjunction with the [`And`] combinator:
    ///
    /// ```
    /// use tracing_subscriber::{
    ///     filter::{filter_fn, LevelFilter, combinator};
    ///     prelude::*,
    /// };
    ///
    /// // This filter will enable spans and events with targets beginning with
    /// // `my_crate`:
    /// let my_crate =  target_filter = filter::filter_fn(|meta| {
    ///     meta.target().starts_with("my_crate")
    /// });
    ///
    /// // Combine the `my_crate` filter with a `LevelFilter` to produce a filter
    /// // that will enable the `INFO` level and lower for spans and events with
    /// // `my_crate` targets:
    /// let filter = combinator::And::new(my_crate, LevelFilter::INFO);
    ///
    /// // If a span or event *doesn't* have a target beginning with
    /// // `my_crate`, enable it if it has the `WARN` level or lower:
    /// // let filter = combinator::Or::new(filter, LevelFilter::WARN);
    ///
    /// tracing_subscriber::registry()
    ///     .with(tracing_subscriber::fmt::layer().with_filter(filter))
    ///     .init();
    /// ```
    ///
    /// [`Filter`]: crate::layer::Filter
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
    /// Inverts the result of a [`Filter`].
    ///
    /// If the wrapped filter would enable a span or event, it will be disabled. If
    /// it would disable a span or event, that span or event will be enabled.
    ///
    /// [`Filter`]: crate::layer::Filter
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
