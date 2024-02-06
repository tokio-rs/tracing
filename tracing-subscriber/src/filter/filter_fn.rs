use crate::{
    filter::LevelFilter,
    subscribe::{Context, Subscribe},
};
use core::{any::type_name, fmt, marker::PhantomData};
use tracing_core::{Collect, Interest, Metadata};

/// A filter implemented by a closure or function pointer that
/// determines whether a given span or event is enabled, based on its
/// [`Metadata`].
///
/// This type can be used for both [per-subscriber filtering][plf] (using its
/// [`Filter`] implementation) and [global filtering][global] (using its
/// [`Subscribe`] implementation).
///
/// See the [documentation on filtering with subscribers][filtering] for details.
///
/// [`Metadata`]: tracing_core::Metadata
/// [`Filter`]: crate::subscribe::Filter
/// [`Subscribe`]: crate::subscribe::Subscribe
/// [plf]: crate::subscribe#per-subscriber-filtering
/// [global]: crate::subscribe#global-filtering
/// [filtering]: crate::subscribe#filtering-with-subscribers
#[derive(Clone)]
pub struct FilterFn<F = fn(&Metadata<'_>) -> bool> {
    enabled: F,
    max_level_hint: Option<LevelFilter>,
}

/// A filter implemented by a closure or function pointer that
/// determines whether a given span or event is enabled _dynamically_,
/// potentially based on the current [span context].
///
/// This type can be used for both [per-subscriber filtering][plf] (using its
/// [`Filter`] implementation) and [global filtering][global] (using its
/// [`Subscribe`] implementation).
///
/// See the [documentation on filtering with subscribers][filtering] for details.
///
/// [span context]: crate::subscribe::Context
/// [`Filter`]: crate::subscribe::Filter
/// [`Subscribe`]: crate::subscribe::Subscribe
/// [plf]: crate::subscribe#per-subscriber-filtering
/// [global]: crate::subscribe#global-filtering
/// [filtering]: crate::subscribe#filtering-with-subscribers
pub struct DynFilterFn<
    C,
    // TODO(eliza): should these just be boxed functions?
    F = fn(&Metadata<'_>, &Context<'_, C>) -> bool,
    R = fn(&'static Metadata<'static>) -> Interest,
> {
    enabled: F,
    register_callsite: Option<R>,
    max_level_hint: Option<LevelFilter>,
    _s: PhantomData<fn(C)>,
}

// === impl FilterFn ===

/// Constructs a [`FilterFn`], from a function or closure that returns `true` if
/// a span or event should be enabled, based on its [`Metadata`].
///
/// The returned [`FilterFn`] can be used for both [per-subscriber filtering][plf]
/// (using its [`Filter`] implementation) and [global filtering][global] (using
/// its  [`Subscribe`] implementation).
///
/// See the [documentation on filtering with subscribers][filtering] for details.
///
/// This is equivalent to calling [`FilterFn::new`].
///
/// [`Metadata`]: tracing_core::Metadata
/// [`Filter`]: crate::subscribe::Filter
/// [`Subscribe`]: crate::subscribe::Subscribe
/// [plf]: crate::subscribe#per-subscriber-filtering
/// [global]: crate::subscribe#global-filtering
/// [filtering]: crate::subscribe#filtering-with-subscribers
///
/// # Examples
///
/// ```
/// use tracing_subscriber::{
///     subscribe::{Subscribe, CollectExt},
///     filter,
///     util::SubscriberInitExt,
/// };
///
/// let my_filter = filter::filter_fn(|metadata| {
///     // Only enable spans or events with the target "interesting_things"
///     metadata.target() == "interesting_things"
/// });
///
/// let my_subscriber = tracing_subscriber::fmt::subscriber();
///
/// tracing_subscriber::registry()
///     .with(my_subscriber.with_filter(my_filter))
///     .init();
///
/// // This event will not be enabled.
/// tracing::warn!("something important but uninteresting happened!");
///
/// // This event will be enabled.
/// tracing::debug!(target: "interesting_things", "an interesting minor detail...");
/// ```
pub fn filter_fn<F>(f: F) -> FilterFn<F>
where
    F: Fn(&Metadata<'_>) -> bool,
{
    FilterFn::new(f)
}

/// Constructs a [`DynFilterFn`] from a function or closure that returns `true`
/// if a span or event should be enabled within a particular [span context][`Context`].
///
/// This is equivalent to calling [`DynFilterFn::new`].
///
/// Unlike [`filter_fn`], this function takes a closure or function pointer
/// taking the [`Metadata`] for a span or event *and* the current [`Context`].
/// This means that a [`DynFilterFn`] can choose whether to enable spans or
/// events based on information about the _current_ span (or its parents).
///
/// If this is *not* necessary, use [`filter_fn`] instead.
///
/// The returned [`DynFilterFn`] can be used for both [per-subscriber filtering][plf]
/// (using its [`Filter`] implementation) and [global filtering][global] (using
/// its  [`Subscribe`] implementation).
///
/// See the [documentation on filtering with subscribers][filtering] for details.
///
/// # Examples
///
/// ```
/// use tracing_subscriber::{
///     subscribe::{Subscribe, CollectExt},
///     filter,
///     util::SubscriberInitExt,
/// };
///
/// // Only enable spans or events within a span named "interesting_span".
/// let my_filter = filter::dynamic_filter_fn(|metadata, cx| {
///     // If this *is* "interesting_span", make sure to enable it.
///     if metadata.is_span() && metadata.name() == "interesting_span" {
///         return true;
///     }
///
///     // Otherwise, are we in an interesting span?
///     if let Some(current_span) = cx.lookup_current() {
///         return current_span.name() == "interesting_span";
///     }
///
///     false
/// });
///
/// let my_subscriber = tracing_subscriber::fmt::subscriber();
///
/// tracing_subscriber::registry()
///     .with(my_subscriber.with_filter(my_filter))
///     .init();
///
/// // This event will not be enabled.
/// tracing::info!("something happened");
///
/// tracing::info_span!("interesting_span").in_scope(|| {
///     // This event will be enabled.
///     tracing::debug!("something else happened");
/// });
/// ```
///
/// [`Filter`]: crate::subscribe::Filter
/// [`Subscribe`]: crate::subscribe::Subscribe
/// [plf]: crate::subscribe#per-subscriber-filtering
/// [global]: crate::subscribe#global-filtering
/// [filtering]: crate::subscribe#filtering-with-subscribers
/// [`Context`]: crate::subscribe::Context
/// [`Metadata`]: tracing_core::Metadata
pub fn dynamic_filter_fn<C, F>(f: F) -> DynFilterFn<C, F>
where
    F: Fn(&Metadata<'_>, &Context<'_, C>) -> bool,
{
    DynFilterFn::new(f)
}

impl<F> FilterFn<F>
where
    F: Fn(&Metadata<'_>) -> bool,
{
    /// Constructs a [`FilterFn`] from a function or closure that returns `true`
    /// if a span or event should be enabled, based on its [`Metadata`].
    ///
    /// If determining whether a span or event should be enabled also requires
    /// information about the current span context, use [`DynFilterFn`] instead.
    ///
    /// See the [documentation on per-subscriber filtering][plf] for details on using
    /// [`Filter`]s.
    ///
    /// [`Filter`]: crate::subscribe::Filter
    /// [plf]: crate::subscribe#per-subscriber-filtering
    /// [`Metadata`]: tracing_core::Metadata
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_subscriber::{
    ///     subscribe::{Subscribe, CollectExt},
    ///     filter::FilterFn,
    ///     util::SubscriberInitExt,
    /// };
    ///
    /// let my_filter = FilterFn::new(|metadata| {
    ///     // Only enable spans or events with the target "interesting_things"
    ///     metadata.target() == "interesting_things"
    /// });
    ///
    /// let my_subscriber = tracing_subscriber::fmt::subscriber();
    ///
    /// tracing_subscriber::registry()
    ///     .with(my_subscriber.with_filter(my_filter))
    ///     .init();
    ///
    /// // This event will not be enabled.
    /// tracing::warn!("something important but uninteresting happened!");
    ///
    /// // This event will be enabled.
    /// tracing::debug!(target: "interesting_things", "an interesting minor detail...");
    /// ```
    pub fn new(enabled: F) -> Self {
        Self {
            enabled,
            max_level_hint: None,
        }
    }

    /// Sets the highest verbosity [`Level`] the filter function will enable.
    ///
    /// The value passed to this method will be returned by this `FilterFn`'s
    /// [`Filter::max_level_hint`] method.
    ///
    /// If the provided function will not enable all levels, it is recommended
    /// to call this method to configure it with the most verbose level it will
    /// enable.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_subscriber::{
    ///     subscribe::{Subscribe, CollectExt},
    ///     filter::{filter_fn, LevelFilter},
    ///     util::SubscriberInitExt,
    /// };
    /// use tracing_core::Level;
    ///
    /// let my_filter = filter_fn(|metadata| {
    ///     // Only enable spans or events with targets starting with `my_crate`
    ///     // and levels at or below `INFO`.
    ///     metadata.level() <= &Level::INFO && metadata.target().starts_with("my_crate")
    /// })
    ///     // Since the filter closure will only enable the `INFO` level and
    ///     // below, set the max level hint
    ///     .with_max_level_hint(LevelFilter::INFO);
    ///
    /// let my_subscriber = tracing_subscriber::fmt::subscriber();
    ///
    /// tracing_subscriber::registry()
    ///     .with(my_subscriber.with_filter(my_filter))
    ///     .init();
    /// ```
    ///
    /// [`Level`]: tracing_core::Level
    /// [`Filter::max_level_hint`]: crate::subscribe::Filter::max_level_hint
    pub fn with_max_level_hint(self, max_level_hint: impl Into<LevelFilter>) -> Self {
        Self {
            max_level_hint: Some(max_level_hint.into()),
            ..self
        }
    }

    #[inline]
    pub(in crate::filter) fn is_enabled(&self, metadata: &Metadata<'_>) -> bool {
        let enabled = (self.enabled)(metadata);
        debug_assert!(
            !enabled || self.is_below_max_level(metadata),
            "FilterFn<{}> claimed it would only enable {:?} and below, \
            but it enabled metadata with the {:?} level\nmetadata={:#?}",
            type_name::<F>(),
            self.max_level_hint.unwrap(),
            metadata.level(),
            metadata,
        );

        enabled
    }

    #[inline]
    pub(in crate::filter) fn is_callsite_enabled(
        &self,
        metadata: &'static Metadata<'static>,
    ) -> Interest {
        // Because `self.enabled` takes a `Metadata` only (and no `Context`
        // parameter), we can reasonably assume its results are cacheable, and
        // just return `Interest::always`/`Interest::never`.
        if (self.enabled)(metadata) {
            debug_assert!(
                self.is_below_max_level(metadata),
                "FilterFn<{}> claimed it was only interested in {:?} and below, \
                but it enabled metadata with the {:?} level\nmetadata={:#?}",
                type_name::<F>(),
                self.max_level_hint.unwrap(),
                metadata.level(),
                metadata,
            );
            return Interest::always();
        }

        Interest::never()
    }

    fn is_below_max_level(&self, metadata: &Metadata<'_>) -> bool {
        self.max_level_hint
            .as_ref()
            .map(|hint| metadata.level() <= hint)
            .unwrap_or(true)
    }
}

impl<C, F> Subscribe<C> for FilterFn<F>
where
    F: Fn(&Metadata<'_>) -> bool + 'static,
    C: Collect,
{
    fn enabled(&self, metadata: &Metadata<'_>, _: Context<'_, C>) -> bool {
        self.is_enabled(metadata)
    }

    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        self.is_callsite_enabled(metadata)
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        self.max_level_hint
    }
}

impl<F> From<F> for FilterFn<F>
where
    F: Fn(&Metadata<'_>) -> bool,
{
    fn from(enabled: F) -> Self {
        Self::new(enabled)
    }
}

impl<F> fmt::Debug for FilterFn<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilterFn")
            .field("enabled", &format_args!("{}", type_name::<F>()))
            .field("max_level_hint", &self.max_level_hint)
            .finish()
    }
}

// === impl DynFilterFn ==

impl<C, F> DynFilterFn<C, F>
where
    F: Fn(&Metadata<'_>, &Context<'_, C>) -> bool,
{
    /// Constructs a [`Filter`] from a function or closure that returns `true`
    /// if a span or event should be enabled in the current [span
    /// context][`Context`].
    ///
    /// Unlike [`FilterFn`], a `DynFilterFn` is constructed from a closure or
    /// function pointer that takes both the [`Metadata`] for a span or event
    /// *and* the current [`Context`]. This means that a [`DynFilterFn`] can
    /// choose whether to enable spans or events based on information about the
    /// _current_ span (or its parents).
    ///
    /// If this is *not* necessary, use [`FilterFn`] instead.
    ///
    /// See the [documentation on per-subscriber filtering][plf] for details on using
    /// [`Filter`]s.
    ///
    /// [`Filter`]: crate::subscribe::Filter
    /// [plf]: crate::subscribe#per-subscriber-filtering
    /// [`Context`]: crate::subscribe::Context
    /// [`Metadata`]: tracing_core::Metadata
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_subscriber::{
    ///     subscribe::{Subscribe, CollectExt},
    ///     filter::DynFilterFn,
    ///     util::SubscriberInitExt,
    /// };
    ///
    /// // Only enable spans or events within a span named "interesting_span".
    /// let my_filter = DynFilterFn::new(|metadata, cx| {
    ///     // If this *is* "interesting_span", make sure to enable it.
    ///     if metadata.is_span() && metadata.name() == "interesting_span" {
    ///         return true;
    ///     }
    ///
    ///     // Otherwise, are we in an interesting span?
    ///     if let Some(current_span) = cx.lookup_current() {
    ///         return current_span.name() == "interesting_span";
    ///     }
    ///
    ///     false
    /// });
    ///
    /// let my_subscriber = tracing_subscriber::fmt::subscriber();
    ///
    /// tracing_subscriber::registry()
    ///     .with(my_subscriber.with_filter(my_filter))
    ///     .init();
    ///
    /// // This event will not be enabled.
    /// tracing::info!("something happened");
    ///
    /// tracing::info_span!("interesting_span").in_scope(|| {
    ///     // This event will be enabled.
    ///     tracing::debug!("something else happened");
    /// });
    /// ```
    pub fn new(enabled: F) -> Self {
        Self {
            enabled,
            register_callsite: None,
            max_level_hint: None,
            _s: PhantomData,
        }
    }
}

impl<C, F, R> DynFilterFn<C, F, R>
where
    F: Fn(&Metadata<'_>, &Context<'_, C>) -> bool,
{
    /// Sets the highest verbosity [`Level`] the filter function will enable.
    ///
    /// The value passed to this method will be returned by this `DynFilterFn`'s
    /// [`Filter::max_level_hint`] method.
    ///
    /// If the provided function will not enable all levels, it is recommended
    /// to call this method to configure it with the most verbose level it will
    /// enable.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_subscriber::{
    ///     subscribe::{Subscribe, CollectExt},
    ///     filter::{DynFilterFn, LevelFilter},
    ///     util::SubscriberInitExt,
    /// };
    /// use tracing_core::Level;
    ///
    /// // Only enable spans or events with levels at or below `INFO`, if
    /// // we are inside a span called "interesting_span".
    /// let my_filter = DynFilterFn::new(|metadata, cx| {
    ///     // If the level is greater than INFO, disable it.
    ///     if metadata.level() > &Level::INFO {
    ///         return false;
    ///     }
    ///
    ///     // If any span in the current scope is named "interesting_span",
    ///     // enable this span or event.
    ///     for span in cx.lookup_current().iter().flat_map(|span| span.scope()) {
    ///         if span.name() == "interesting_span" {
    ///             return true;
    ///          }
    ///     }
    ///
    ///     // Otherwise, disable it.
    ///     false
    /// })
    ///     // Since the filter closure will only enable the `INFO` level and
    ///     // below, set the max level hint
    ///     .with_max_level_hint(LevelFilter::INFO);
    ///
    /// let my_subscriber = tracing_subscriber::fmt::subscriber();
    ///
    /// tracing_subscriber::registry()
    ///     .with(my_subscriber.with_filter(my_filter))
    ///     .init();
    /// ```
    ///
    /// [`Level`]: tracing_core::Level
    /// [`Filter::max_level_hint`]: crate::subscribe::Filter::max_level_hint
    pub fn with_max_level_hint(self, max_level_hint: impl Into<LevelFilter>) -> Self {
        Self {
            max_level_hint: Some(max_level_hint.into()),
            ..self
        }
    }

    /// Adds a function for filtering callsites to this filter.
    ///
    /// When this filter's [`Filter::callsite_enabled`][cse] method is called,
    /// the provided function will be used rather than the default.
    ///
    /// By default, `DynFilterFn` assumes that, because the filter _may_ depend
    /// dynamically on the current [span context], its result should never be
    /// cached. However, some filtering strategies may require dynamic information
    /// from the current span context in *some* cases, but are able to make
    /// static filtering decisions from [`Metadata`] alone in others.
    ///
    /// For example, consider the filter given in the example for
    /// [`DynFilterFn::new`]. That filter enables all spans named
    /// "interesting_span", and any events and spans that occur inside of an
    /// interesting span. Since the span's name is part of its static
    /// [`Metadata`], the "interesting_span" can be enabled in
    /// [`callsite_enabled`][cse]:
    ///
    /// ```
    /// use tracing_subscriber::{
    ///     subscribe::{Subscribe, CollectExt},
    ///     filter::DynFilterFn,
    ///     util::SubscriberInitExt,
    /// };
    /// use tracing_core::collect::Interest;
    ///
    /// // Only enable spans or events within a span named "interesting_span".
    /// let my_filter = DynFilterFn::new(|metadata, cx| {
    ///     // If this *is* "interesting_span", make sure to enable it.
    ///     if metadata.is_span() && metadata.name() == "interesting_span" {
    ///         return true;
    ///     }
    ///
    ///     // Otherwise, are we in an interesting span?
    ///     if let Some(current_span) = cx.lookup_current() {
    ///         return current_span.name() == "interesting_span";
    ///     }
    ///
    ///     false
    /// }).with_callsite_filter(|metadata| {
    ///     // If this is an "interesting_span", we know we will always
    ///     // enable it.
    ///     if metadata.is_span() && metadata.name() == "interesting_span" {
    ///         return Interest::always();
    ///     }
    ///
    ///     // Otherwise, it depends on whether or not we're in an interesting
    ///     // span. You'll have to ask us again for each span/event!
    ///     Interest::sometimes()
    /// });
    ///
    /// let my_subscriber = tracing_subscriber::fmt::subscriber();
    ///
    /// tracing_subscriber::registry()
    ///     .with(my_subscriber.with_filter(my_filter))
    ///     .init();
    /// ```
    ///
    /// [cse]: crate::subscribe::Filter::callsite_enabled
    /// [`enabled`]: crate::subscribe::Filter::enabled
    /// [`Metadata`]: tracing_core::Metadata
    /// [span context]: crate::subscribe::Context
    pub fn with_callsite_filter<R2>(self, callsite_enabled: R2) -> DynFilterFn<C, F, R2>
    where
        R2: Fn(&'static Metadata<'static>) -> Interest,
    {
        let register_callsite = Some(callsite_enabled);
        let DynFilterFn {
            enabled,
            max_level_hint,
            _s,
            ..
        } = self;
        DynFilterFn {
            enabled,
            register_callsite,
            max_level_hint,
            _s,
        }
    }

    fn default_callsite_enabled(&self, metadata: &Metadata<'_>) -> Interest {
        // If it's below the configured max level, assume that `enabled` will
        // never enable it...
        if !is_below_max_level(&self.max_level_hint, metadata) {
            debug_assert!(
                !(self.enabled)(metadata, &Context::none()),
                "DynFilterFn<{}> claimed it would only enable {:?} and below, \
                but it enabled metadata with the {:?} level\nmetadata={:#?}",
                type_name::<F>(),
                self.max_level_hint.unwrap(),
                metadata.level(),
                metadata,
            );
            return Interest::never();
        }

        // Otherwise, since this `enabled` function is dynamic and depends on
        // the current context, we don't know whether this span or event will be
        // enabled or not. Ask again every time it's recorded!
        Interest::sometimes()
    }
}

impl<C, F, R> DynFilterFn<C, F, R>
where
    F: Fn(&Metadata<'_>, &Context<'_, C>) -> bool,
    R: Fn(&'static Metadata<'static>) -> Interest,
{
    #[inline]
    fn is_enabled(&self, metadata: &Metadata<'_>, cx: &Context<'_, C>) -> bool {
        let enabled = (self.enabled)(metadata, cx);
        debug_assert!(
            !enabled || is_below_max_level(&self.max_level_hint, metadata),
            "DynFilterFn<{}> claimed it would only enable {:?} and below, \
            but it enabled metadata with the {:?} level\nmetadata={:#?}",
            type_name::<F>(),
            self.max_level_hint.unwrap(),
            metadata.level(),
            metadata,
        );

        enabled
    }

    #[inline]
    fn is_callsite_enabled(&self, metadata: &'static Metadata<'static>) -> Interest {
        let interest = self
            .register_callsite
            .as_ref()
            .map(|callsite_enabled| callsite_enabled(metadata))
            .unwrap_or_else(|| self.default_callsite_enabled(metadata));
        debug_assert!(
            interest.is_never() || is_below_max_level(&self.max_level_hint, metadata),
            "DynFilterFn<{}, {}> claimed it was only interested in {:?} and below, \
            but it enabled metadata with the {:?} level\nmetadata={:#?}",
            type_name::<F>(),
            type_name::<R>(),
            self.max_level_hint.unwrap(),
            metadata.level(),
            metadata,
        );

        interest
    }
}

impl<C, F, R> Subscribe<C> for DynFilterFn<C, F, R>
where
    F: Fn(&Metadata<'_>, &Context<'_, C>) -> bool + 'static,
    R: Fn(&'static Metadata<'static>) -> Interest + 'static,
    C: Collect,
{
    fn enabled(&self, metadata: &Metadata<'_>, cx: Context<'_, C>) -> bool {
        self.is_enabled(metadata, &cx)
    }

    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        self.is_callsite_enabled(metadata)
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        self.max_level_hint
    }
}

impl<C, F, R> fmt::Debug for DynFilterFn<C, F, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("DynFilterFn");
        s.field("enabled", &format_args!("{}", type_name::<F>()));
        if self.register_callsite.is_some() {
            s.field(
                "register_callsite",
                &format_args!("Some({})", type_name::<R>()),
            );
        } else {
            s.field("register_callsite", &format_args!("None"));
        }

        s.field("max_level_hint", &self.max_level_hint).finish()
    }
}

impl<C, F, R> Clone for DynFilterFn<C, F, R>
where
    F: Clone,
    R: Clone,
{
    fn clone(&self) -> Self {
        Self {
            enabled: self.enabled.clone(),
            register_callsite: self.register_callsite.clone(),
            max_level_hint: self.max_level_hint,
            _s: PhantomData,
        }
    }
}

impl<F, C> From<F> for DynFilterFn<C, F>
where
    F: Fn(&Metadata<'_>, &Context<'_, C>) -> bool,
{
    fn from(f: F) -> Self {
        Self::new(f)
    }
}

// === PLF impls ===

feature! {
    #![all(feature = "registry", feature = "std")]
    use crate::subscribe::Filter;

    impl<C, F> Filter<C> for FilterFn<F>
    where
        F: Fn(&Metadata<'_>) -> bool,
    {
        fn enabled(&self, metadata: &Metadata<'_>, _: &Context<'_, C>) -> bool {
            self.is_enabled(metadata)
        }

        fn callsite_enabled(&self, metadata: &'static Metadata<'static>) -> Interest {
            self.is_callsite_enabled(metadata)
        }

        fn max_level_hint(&self) -> Option<LevelFilter> {
            self.max_level_hint
        }
    }

    impl<C, F, R> Filter<C> for DynFilterFn<C, F, R>
    where
        F: Fn(&Metadata<'_>, &Context<'_, C>) -> bool,
        R: Fn(&'static Metadata<'static>) -> Interest,
    {
        fn enabled(&self, metadata: &Metadata<'_>, cx: &Context<'_, C>) -> bool {
            self.is_enabled(metadata, cx)
        }

        fn callsite_enabled(&self, metadata: &'static Metadata<'static>) -> Interest {
            self.is_callsite_enabled(metadata)
        }

        fn max_level_hint(&self) -> Option<LevelFilter> {
            self.max_level_hint
        }
    }
}

fn is_below_max_level(hint: &Option<LevelFilter>, metadata: &Metadata<'_>) -> bool {
    hint.as_ref()
        .map(|hint| metadata.level() <= hint)
        .unwrap_or(true)
}
