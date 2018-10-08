use filter::{self, Filter};
use tokio_trace::{Event, Meta, SpanData};

/// The notification processing portion of the [`Subscriber`] trait.
///
/// Implementations of this trait describe the logic needed to process envent
/// and span notifications, but don't implement span registration.
pub trait Observe {
    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>);
    fn enter(&self, span: &SpanData);
    fn exit(&self, span: &SpanData);

    fn filter(&self) -> &dyn Filter {
        &filter::NoFilter
    }
}

/// Extension trait providing combinators and helper methods for working with
/// instances of `Observe`.
pub trait ObserveExt: Observe {
    /// Construct a new observer that sends events to both `self` and `other`.
    ///
    /// For example:
    /// ```
    /// #[macro_use]
    /// extern crate tokio_trace;
    /// extern crate tokio_trace_log;
    /// extern crate tokio_trace_subscriber;
    /// use tokio_trace_subscriber::{registry, filter, Observe, ObserveExt};
    /// # use tokio_trace::{Level, Meta};
    /// # fn main() {
    ///
    /// let observer = tokio_trace_log::TraceLogger::new()
    ///     // Subscribe *only* to spans named "foo".
    ///     .with_filter(|meta: &Meta| {
    ///         meta.name == Some("foo")
    ///     });
    ///
    /// let subscriber = tokio_trace_subscriber::Composed::builder()
    ///     .with_observer(observer)
    ///     .with_registry(registry::increasing_counter);
    ///
    /// tokio_trace::Dispatch::to(subscriber).with(|| {
    ///     /// // This span will be logged.
    ///     span!("foo", enabled = true) .enter(|| {
    ///         // do work;
    ///     });
    ///     // This span will *not* be logged.
    ///     span!("bar", enabled = false).enter(|| {
    ///         // This event also will not be logged.
    ///         event!(Level::Debug, { enabled = false },"this won't be logged");
    ///     });
    /// });
    /// # }
    /// ```
    ///
    fn tee_to<I>(self, other: I) -> Tee<Self, I::Observer>
    where
        I: IntoObserver,
        Self: Sized,
    {
        Tee {
            a: self,
            b: other.into_observer(),
        }
    }

    /// Composes `self` with a [`Filter`].
    ///
    /// This function is intended to be used with composing observers from
    /// external crates with user-defined filters, so that the resulting
    /// observer is [`enabled`] only for a subset of the events and spans for
    /// which the original observer would be enabled.
    ///
    /// For example:
    /// ```
    /// #[macro_use]
    /// extern crate tokio_trace;
    /// extern crate tokio_trace_subscriber;
    /// use tokio_trace_subscriber::{registry, Observe, ObserveExt};
    /// # use tokio_trace_subscriber::filter::{Filter, NoFilter};
    /// # use tokio_trace::{Level, Meta, Event, SpanData};
    /// # fn main() {
    ///
    /// struct Foo {
    ///     // ...
    /// }
    ///
    /// struct Bar {
    ///     // ...
    /// }
    ///
    /// impl Observe for Foo {
    ///     // ...
    /// # fn observe_event<'event, 'meta: 'event>(&self, _: &'event Event<'event, 'meta>) {}
    /// # fn enter(&self, _: &SpanData) {}
    /// # fn exit(&self, _: &SpanData) {}
    /// # fn filter(&self) -> &dyn Filter { &NoFilter}
    /// }
    ///
    /// impl Observe for Bar {
    ///     // ...
    /// # fn observe_event<'event, 'meta: 'event>(&self, _: &'event Event<'event, 'meta>) {}
    /// # fn enter(&self, _: &SpanData) {}
    /// # fn exit(&self, _: &SpanData) {}
    /// # fn filter(&self) -> &dyn Filter { &NoFilter}
    /// }
    ///
    /// let foo = Foo { };
    /// let bar = Bar { };
    ///
    /// let observer = foo.tee_to(bar);
    ///
    /// let subscriber = tokio_trace_subscriber::Composed::builder()
    ///     .with_observer(observer)
    ///     .with_registry(registry::increasing_counter);
    ///
    /// tokio_trace::Dispatch::to(subscriber).with(|| {
    ///     // This span will be seen by both `foo` and `bar`.
    ///     span!("my great span").enter(|| {
    ///         // ...
    ///     })
    /// });
    /// # }
    /// ```
    ///
    /// [`Filter`]: ../trait.Filter.html
    /// [`enabled`]: ../trait.Filter.html#tymethod.enabled
    fn with_filter<F>(self, filter: F) -> WithFilter<Self, F>
    where
        F: Filter,
        Self: Sized,
    {
        WithFilter {
            inner: self,
            filter,
        }
    }
}

pub trait IntoObserver {
    type Observer: Observe;
    fn into_observer(self) -> Self::Observer;
}

/// An observer which does nothing.
pub struct NoObserver;

/// An observer which is an instance of one of two types that implement
/// `Observe`.
///
/// This is intended to be used when an observer implementation is chosen
/// conditionally, and the overhead of `Box<dyn Observe>` is unwanted.
///
/// For example:
/// ```
/// # extern crate tokio_trace;
/// extern crate tokio_trace_subscriber;
/// use tokio_trace_subscriber::{observe, Observe};
/// # use tokio_trace_subscriber::filter::{Filter, NoFilter};
/// # use tokio_trace::{Event, SpanData};
/// # fn main() {}
///
/// struct Foo {
///     // ...
/// }
///
/// struct Bar {
///     // ...
/// }
///
/// impl Observe for Foo {
///     // ...
/// # fn observe_event<'event, 'meta: 'event>(&self, _: &'event Event<'event, 'meta>) {}
/// # fn enter(&self, _: &SpanData) {}
/// # fn exit(&self, _: &SpanData) {}
/// # fn filter(&self) -> &dyn Filter { &NoFilter}
/// }
///
/// impl Observe for Bar {
///     // ...
/// # fn observe_event<'event, 'meta: 'event>(&self, _: &'event Event<'event, 'meta>) {}
/// # fn enter(&self, _: &SpanData) {}
/// # fn exit(&self, _: &SpanData) {}
/// # fn filter(&self) -> &dyn Filter { &NoFilter}
/// }
///
/// fn foo_or_bar(foo: bool) -> observe::Either<Foo, Bar> {
///     if foo {
///         observe::Either::A(Foo { })
///     } else {
///         observe::Either::B(Bar { })
///     }
/// }
/// ```
#[derive(Copy, Clone)]
pub enum Either<A, B> {
    A(A),
    B(B),
}

/// An observer that forwards events and spans to two other types implementing
/// `Observe`.
///
/// The `Tee`'s filter composes the filters of its child observers, so that a
/// span or event is enabled if either of the child observers' filters consider
/// it enabled. Similarly, cached filter evaluations should be invalidated if
/// either child observer's filter indicates that they should be.
#[derive(Copy, Clone)]
pub struct Tee<A, B> {
    a: A,
    b: B,
}

/// An observer composed with an additional filter.
///
/// This observer's filter considers a span or event enabled if **both** the
/// wrapped observer's filter and the composed filter enable it. However, cached
/// filters are invalidated if **either** filter indicates that they should be.
#[derive(Debug, Clone)]
pub struct WithFilter<O, F> {
    inner: O,
    filter: F,
}

impl<O, F> Filter for WithFilter<O, F>
where
    O: Observe,
    F: Filter,
{
    #[inline]
    fn enabled(&self, metadata: &Meta) -> bool {
        self.filter.enabled(metadata) && self.inner.filter().enabled(metadata)
    }

    #[inline]
    fn should_invalidate_filter(&self, metadata: &Meta) -> bool {
        self.filter.should_invalidate_filter(metadata)
            || self.inner.filter().should_invalidate_filter(metadata)
    }
}

impl<O, F> Observe for WithFilter<O, F>
where
    O: Observe,
    F: Filter,
{
    #[inline]
    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {
        self.inner.observe_event(event)
    }

    #[inline]
    fn enter(&self, span: &SpanData) {
        self.inner.enter(span)
    }

    #[inline]
    fn exit(&self, span: &SpanData) {
        self.inner.exit(span)
    }

    fn filter(&self) -> &dyn Filter {
        self
    }
}

pub fn none() -> NoObserver {
    NoObserver
}

impl<T> ObserveExt for T where T: Observe {}

impl<T> IntoObserver for T
where
    T: Observe,
{
    type Observer = Self;
    fn into_observer(self) -> Self::Observer {
        self
    }
}

// XXX: maybe this should just be an impl of `Observe` for tuples of `(Observe, Observe)`...?
impl<A, B> Observe for Tee<A, B>
where
    A: Observe,
    B: Observe,
{
    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {
        self.a.observe_event(event);
        self.b.observe_event(event);
    }

    fn enter(&self, span: &SpanData) {
        self.a.enter(span);
        self.b.enter(span);
    }

    fn exit(&self, span: &SpanData) {
        self.a.exit(span);
        self.b.exit(span);
    }

    fn filter(&self) -> &dyn Filter {
        self
    }
}

impl<A, B> Filter for Tee<A, B>
where
    A: Observe,
    B: Observe,
{
    fn enabled(&self, metadata: &Meta) -> bool {
        self.a.filter().enabled(metadata) || self.b.filter().enabled(metadata)
    }

    fn should_invalidate_filter(&self, metadata: &Meta) -> bool {
        self.a.filter().should_invalidate_filter(metadata)
            || self.b.filter().should_invalidate_filter(metadata)
    }
}

impl<A, B> Observe for Either<A, B>
where
    A: Observe,
    B: Observe,
{
    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {
        match self {
            Either::A(a) => a.observe_event(event),
            Either::B(b) => b.observe_event(event),
        }
    }

    fn enter(&self, span: &SpanData) {
        match self {
            Either::A(a) => a.enter(span),
            Either::B(b) => b.enter(span),
        }
    }

    fn exit(&self, span: &SpanData) {
        match self {
            Either::A(a) => a.exit(span),
            Either::B(b) => b.exit(span),
        }
    }
}

impl<A, B> Filter for Either<A, B>
where
    A: Observe,
    B: Observe,
{
    fn enabled(&self, metadata: &Meta) -> bool {
        match self {
            Either::A(a) => a.filter().enabled(metadata),
            Either::B(b) => b.filter().enabled(metadata),
        }
    }

    fn should_invalidate_filter(&self, metadata: &Meta) -> bool {
        match self {
            Either::A(a) => a.filter().should_invalidate_filter(metadata),
            Either::B(b) => b.filter().should_invalidate_filter(metadata),
        }
    }
}

impl Observe for NoObserver {
    fn observe_event<'event, 'meta: 'event>(&self, _event: &'event Event<'event, 'meta>) {}

    fn enter(&self, _span: &SpanData) {}

    fn exit(&self, _span: &SpanData) {}

    fn filter(&self) -> &dyn Filter {
        self
    }
}

impl Filter for NoObserver {
    fn enabled(&self, _metadata: &Meta) -> bool {
        false
    }

    fn should_invalidate_filter(&self, _metadata: &Meta) -> bool {
        false
    }
}
