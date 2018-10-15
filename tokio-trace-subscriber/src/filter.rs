use tokio_trace::Meta;

use std::{
    collections::HashSet,
    sync::atomic::{AtomicUsize, Ordering},
};

/// The filtering portion of the [`Subscriber`] trait.
///
/// Implementations of this trait represent _just_ the logic necessary to filter
/// events and spans, but none of the processing or registration logic.
pub trait Filter {
    /// Determines if a span or event with the specified metadata would be recorded.
    ///
    /// This is used by the dispatcher to avoid allocating for span construction
    /// if the span would be discarded anyway.
    fn enabled(&self, metadata: &Meta) -> bool;

    /// Returns `true` if the cached result to a call to `enabled` for a span
    /// with the given metadata is still valid.
    ///
    /// By default, this function assumes that cached filter results will remain
    /// valid, but should be overridden when this is not the case.
    ///
    /// If this returns `false`, then the prior value may be used.
    /// `Subscriber`s which require their filters to be run every time an event
    /// occurs or a span is entered/exited should always return `true`.
    ///
    /// For example, suppose a sampling subscriber is implemented by incrementing a
    /// counter every time `enabled` is called and only returning `true` when
    /// the counter is divisible by a specified sampling rate. If that
    /// subscriber returns `false` from `should_invalidate_filter`, then the
    /// filter will not be re-evaluated once it has been applied to a given set
    /// of metadata. Thus, the counter will not be incremented, and the span or
    /// event that correspands to the metadata will never be `enabled`.
    ///
    /// Similarly, if a `Subscriber` has a filtering strategy that can be
    /// changed dynamically at runtime, it would need to invalidate any cached
    /// filter results when the filtering rules change.
    ///
    /// A subscriber which manages fanout to multiple other subscribers should
    /// proxy this decision to all of its child subscribers, returning `false`
    /// only if _all_ such children return `false`. If the set of subscribers to
    /// which spans are broadcast may change dynamically, adding a new
    /// subscriber should also invalidate cached filters.
    fn should_invalidate_filter(&self, metadata: &Meta) -> bool;
}

/// Extension trait providing combinators and helper methods for working with
/// instances of `Filter`.
pub trait FilterExt: Filter {
    /// Construct a new `Filter` that enables a span or event if both `self`
    /// *AND* `other` consider it enabled.
    ///
    /// For example:
    /// ```
    /// #[macro_use]
    /// extern crate tokio_trace;
    /// extern crate tokio_trace_subscriber;
    /// use tokio_trace_subscriber::{Filter, FilterExt};
    /// # use tokio_trace::{Level, Meta};
    /// # fn main() {
    /// fn foo() {
    ///     // This span will not be enabled.
    ///     span!("foo").enter(|| { })
    /// }
    ///
    /// pub mod my_module {
    ///     pub fn foo() {
    ///         // This span will be enabled.
    ///         span!("foo").enter(|| { })
    ///     }
    ///
    ///     pub fn bar() {
    ///         // This span will not enabled.
    ///         span!("foo").enter(|| { })
    ///     }
    /// }
    ///
    /// let name_filter = |meta: &Meta| { meta.name == Some("foo") };
    /// let mod_filter = |meta: &Meta| { meta.module_path == Some("my_module") };
    ///
    /// let subscriber = tokio_trace_subscriber::Composed::builder()
    ///     .with_registry(tokio_trace_subscriber::registry::increasing_counter)
    ///     .with_filter(name_filter.and(mod_filter));
    ///
    /// tokio_trace::Dispatch::to(subscriber).with(|| {
    ///     foo();
    ///     my_module::foo();
    ///     my_module::bar();
    /// });
    ///
    /// # }
    /// ```
    fn and<B>(self, other: B) -> And<Self, B>
    where
        B: Filter + Sized,
        Self: Sized,
    {
        And { a: self, b: other }
    }

    /// Construct a new `Filter` that enables a span or event if either `self`
    /// *OR* `other` consider it enabled.
    fn or<B>(self, other: B) -> Or<Self, B>
    where
        B: Filter + Sized,
        Self: Sized,
    {
        Or { a: self, b: other }
    }
}

#[derive(Debug, Clone)]
pub struct NoFilter;

#[derive(Debug, Clone)]
pub struct And<A, B> {
    a: A,
    b: B,
}

#[derive(Debug, Clone)]
pub struct Or<A, B> {
    a: A,
    b: B,
}

/// A filter that enables some fraction of events.
#[derive(Debug)]
pub struct Sample {
    every: usize,
    count: AtomicUsize,
}

/// A filter that enables all spans and events, except those originating
/// from a specified set of module paths.
#[derive(Debug)]
pub struct ExceptModules {
    modules: HashSet<String>,
}

/// A filter that enables only spans and events originating from a specified
/// set of module paths.
#[derive(Debug)]
pub struct OnlyModules {
    modules: HashSet<String>,
}

/// A filter that enables all spans and events, except those with a specified
/// set of targets.
#[derive(Debug)]
pub struct ExceptTargets {
    targets: HashSet<String>,
}

/// A filter that enables onlu spans and events with a specified set of targets.
#[derive(Debug)]
pub struct OnlyTargets {
    targets: HashSet<String>,
}

/// Returns a filter that enables all spans and events, except those originating
/// from a specified set of module paths.
pub fn except_modules<I>(modules: I) -> ExceptModules
where
    I: IntoIterator,
    String: From<<I as IntoIterator>::Item>,
{
    let modules = modules.into_iter().map(String::from).collect();
    ExceptModules { modules }
}

/// Returns a filter that enables only spans and events originating from a
/// specified set of module paths.
pub fn only_modules<I>(modules: I) -> OnlyModules
where
    I: IntoIterator,
    String: From<<I as IntoIterator>::Item>,
{
    let modules = modules.into_iter().map(String::from).collect();
    OnlyModules { modules }
}

/// A filter which only enables spans.
pub fn spans_only<'a, 'b>(metadata: &'a Meta<'b>) -> bool {
    metadata.is_span()
}

/// A filter which only enables events.
pub fn events_only<'a, 'b>(metadata: &'a Meta<'b>) -> bool {
    metadata.is_event()
}

/// Returns a filter that enables all spans and events, except those originating
/// with a specified set of targets.
pub fn except_targets<I>(targets: I) -> ExceptTargets
where
    I: IntoIterator,
    String: From<<I as IntoIterator>::Item>,
{
    let targets = targets.into_iter().map(String::from).collect();
    ExceptTargets { targets }
}

/// Returns a filter that enables only spans and events with a specified set of
/// targets.
pub fn only_targets<I>(targets: I) -> OnlyTargets
where
    I: IntoIterator,
    String: From<<I as IntoIterator>::Item>,
{
    let targets = targets.into_iter().map(String::from).collect();
    OnlyTargets { targets }
}

impl<F> Filter for F
where
    F: for<'a, 'b> Fn(&'a Meta<'b>) -> bool,
{
    fn enabled(&self, meta: &Meta) -> bool {
        self(meta)
    }

    fn should_invalidate_filter(&self, _: &Meta) -> bool {
        // Since this implementation is for immutable closures only, we can
        // treat these functions as stateless and assume they remain valid.
        false
    }
}

impl<A, B> Filter for And<A, B>
where
    A: Filter,
    B: Filter,
{
    fn enabled(&self, metadata: &Meta) -> bool {
        self.a.enabled(metadata) && self.b.enabled(metadata)
    }

    fn should_invalidate_filter(&self, metadata: &Meta) -> bool {
        // Even though this is the `And` composition, that only applies to the
        // actual filter result, not whether or not the filter needs to be
        // invalidated. If either of the composed filters requests its cached
        // results be invalidated, we need to honor that.
        self.a.should_invalidate_filter(metadata) || self.b.should_invalidate_filter(metadata)
    }
}

impl<A, B> Filter for Or<A, B>
where
    A: Filter,
    B: Filter,
{
    fn enabled(&self, metadata: &Meta) -> bool {
        self.a.enabled(metadata) || self.b.enabled(metadata)
    }

    fn should_invalidate_filter(&self, metadata: &Meta) -> bool {
        self.a.should_invalidate_filter(metadata) || self.b.should_invalidate_filter(metadata)
    }
}

impl Sample {
    /// Construct a new filter that is enabled for every `every` spans/events.
    pub fn every(every: usize) -> Self {
        Self {
            every,
            count: AtomicUsize::new(0),
        }
    }

    // TODO: constructors with ratios, percentages, etc?
}

impl Filter for Sample {
    fn enabled(&self, _metadata: &Meta) -> bool {
        // TODO: it would be nice to be able to have a definition of sampling
        // that also enables all the children of a sampled span...figure that out.
        let current = self.count.fetch_add(1, Ordering::Acquire);
        if current % self.every == 0 {
            self.count.store(0, Ordering::Release);
            true
        } else {
            false
        }
    }

    fn should_invalidate_filter(&self, _metadata: &Meta) -> bool {
        // The filter _needs_ to be re-evaluated every time, or else the counter
        // won't be updated.
        true
    }
}

impl Filter for NoFilter {
    fn enabled(&self, _metadata: &Meta) -> bool {
        true
    }

    fn should_invalidate_filter(&self, _metadata: &Meta) -> bool {
        false
    }
}

impl Filter for ExceptModules {
    fn enabled(&self, metadata: &Meta) -> bool {
        metadata.module_path
            .map(|module| !self.modules.contains(module))
            .unwrap_or(true)

    }

    fn should_invalidate_filter(&self, _metadata: &Meta) -> bool {
        false
    }
}

impl Filter for OnlyModules {
    fn enabled(&self, metadata: &Meta) -> bool {
        metadata.module_path
            .map(|module| self.modules.contains(module))
            .unwrap_or(false)
    }

    fn should_invalidate_filter(&self, _metadata: &Meta) -> bool {
        false
    }
}


impl Filter for ExceptTargets {
    fn enabled(&self, metadata: &Meta) -> bool {
        !self.targets.contains(metadata.target)
    }

    fn should_invalidate_filter(&self, _metadata: &Meta) -> bool {
        false
    }
}

impl Filter for OnlyTargets {
    fn enabled(&self, metadata: &Meta) -> bool {
        self.targets.contains(metadata.target)
    }

    fn should_invalidate_filter(&self, _metadata: &Meta) -> bool {
        false
    }
}

impl<F> FilterExt for F where F: Filter {}
