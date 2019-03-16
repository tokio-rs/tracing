use span;

use std::any::{Any, TypeId};
use tokio_trace_core::{Event, Metadata};

/// The notification processing portion of the [`Subscriber`] trait.
///
/// Implementations of this trait describe the logic needed to process envent
/// and span notifications, but don't implement span registration.
pub trait Observe<R>: 'static
where
    R: for<'a> span::Registry<'a>,
{
    fn event(&self, event: &Event, registry: &R);
    fn enter(&self, id: &span::Id, registry: &R);
    fn exit(&self, id: &span::Id, registry: &R);
    fn record(&self, id: &span::Id, record: &span::Record, registry: &R);
    fn close(&self, id: &span::Id, registry: &R);

    // fn filter(&self) -> &Filter {
    //     &filter::NoFilter
    // }

    // === Downcasting methods ================================================

    /// Gets the `TypeId` of `Self`.
    ///
    /// Implementations of `Subscriber` are **not** expected to override this!
    #[doc(hidden)]
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    /// If `self` is the same type as the provided `TypeId`, returns an untyped
    /// `*const` pointer to that type. Otherwise, returns `None`.
    ///
    /// If you wish to downcast a `Subscriber`, it is strongly advised to use
    /// the safe API provided by [`downcast_ref`] instead.
    ///
    /// This API is required for `downcast_raw` to be a trait method; a method
    /// signature like [`downcast_ref`] (with a generic type parameter) is not
    /// object-safe, and thus cannot be a trait method for `Subscriber`. This
    /// means that if we only exposed `downcast_ref`, `Subscriber`
    /// implementations could not override the downcasting behavior
    ///
    /// This method may be overridden by "fan out" or "chained" subscriber
    /// implementations which consist of multiple composed types. Such
    /// subscribers might allow `downcast_raw` by returning references to those
    /// component if they contain components with the given `TypeId`.
    fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        if id == self.type_id() {
            Some(self as *const Self as *const ())
        } else {
            None
        }
    }

    /// Returns `true` if this `Subscriber` is the same type as `T`.
    fn is<T: Any>(&self) -> bool {
        self.downcast_raw(TypeId::of::<T>()).is_some()
    }

    /// Returns some reference to this `Subscriber` value if it is of type `T`,
    /// or `None` if it isn't.
    fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.downcast_raw(TypeId::of::<T>())
            .map(|raw| unsafe { &*(raw as *const _) })
    }
}

/// Extension trait providing combinators and helper methods for working with
/// instances of `Observe`.
pub trait ObserveExt<R>: Observe<R>
where
    R: for<'a> span::Registry<'a>,
{
    /// Construct a new observer that sends events to both `self` and `other`.
    ///
    /// For example:
    /// ```
    /// #[macro_use]
    /// extern crate tokio_trace;
    /// extern crate tokio_trace_subscriber;
    /// use tokio_trace_subscriber::{registry, Event, Observe, ObserveExt, SpanRef};
    /// # use tokio_trace_subscriber::filter::{Filter, NoFilter};
    /// # use tokio_trace::{Level, Metadata, Span};
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
    /// impl<R> Observe<R> for Foo
    /// where
    ///    R: for<'a> span::Registry<'a>,
    /// {
    ///     // ...
    /// # fn event(&self, _: &Event, _: &R) {}
    /// # fn enter(&self, _: &Id, _: &R) {}
    /// # fn exit(&self, _: &Id, _: &R) {}
    /// # fn close(&self, _: &Id, _: &R) {}
    /// # fn record(&self, id: &span::Id, record: &span::Record, registry: &R);
    /// }
    ///
    /// impl<R> Observe<R> for Bar
    /// where
    ///    R: for<'a> span::Registry<'a>,
    /// {
    ///     // ...
    ///
    /// # fn event(&self, _: &Event, _: &R) {}
    /// # fn enter(&self, _: &Id, _: &R) {}
    /// # fn exit(&self, _: &Id, _: &R) {}
    /// # fn close(&self, _: &Id, _: &R) {}
    /// }
    ///
    /// let foo = Foo { };
    /// let bar = Bar { };
    ///
    /// let observer = foo.tee_to(bar);
    ///
    /// let subscriber = tokio_trace_subscriber::Composed::builder()
    ///     .with_observer(observer)
    ///     .with_registry(registry::increasing_counter());
    ///
    /// tokio_trace::subscriber::with_default(subscriber, || {
    ///     // This span will be seen by both `foo` and `bar`.
    ///     span!("my great span").enter(|| {
    ///         // ...
    ///     })
    /// });
    /// # }
    /// ```
    fn tee_to<I>(self, other: I) -> Tee<Self, I::Observer>
    where
        I: IntoObserver<R>,
        Self: Sized,
    {
        Tee {
            a: self,
            b: other.into_observer(),
        }
    }

    // /// Composes `self` with a [`Filter`].
    // ///
    // /// This function is intended to be used with composing observers from
    // /// external crates with user-defined filters, so that the resulting
    // /// observer is [`enabled`] only for a subset of the events and spans for
    // /// which the original observer would be enabled.
    // ///
    // ///
    // /// For example:
    // // TODO: this needs to be fixed since it uses the `tokio-trace-log` crate,
    // // which  doesn't work with `Observer` yet.
    // /// ```ignore
    // /// #[macro_use]
    // /// extern crate tokio_trace;
    // /// extern crate tokio_trace_log;
    // /// extern crate tokio_trace_subscriber;
    // /// use tokio_trace_subscriber::{registry, filter, Observe, ObserveExt, SpanRef};
    // /// # use tokio_trace::{Level, Metadata, Span};
    // /// # fn main() {
    // ///
    // /// let observer = tokio_trace_log::TraceLogger::new()
    // ///     // Subscribe *only* to spans named "foo".
    // ///     .with_filter(|meta: &Metadata| {
    // ///         meta.name == Some("foo")
    // ///     });
    // ///
    // /// let subscriber = tokio_trace_subscriber::Composed::builder()
    // ///     .with_observer(observer)
    // ///     .with_registry(registry::increasing_counter());
    // ///
    // /// tokio_trace::Dispatch::new(subscriber).as_default(|| {
    // ///     /// // This span will be logged.
    // ///     span!("foo", enabled = &true) .enter(|| {
    // ///         // do work;
    // ///     });
    // ///     // This span will *not* be logged.
    // ///     span!("bar", enabled = &false).enter(|| {
    // ///         // This event also will not be logged.
    // ///         event!(Level::Debug, { enabled = false },"this won't be logged");
    // ///     });
    // /// });
    // /// # }
    // /// ```
    // ///
    // /// [`Filter`]: ../trait.Filter.html
    // /// [`enabled`]: ../trait.Filter.html#tymethod.enabled
    // fn with_filter<F>(self, filter: F) -> WithFilter<Self, F>
    // where
    //     F: Filter,
    //     Self: Sized,
    // {
    //     WithFilter {
    //         inner: self,
    //         filter,
    //     }
    // }
}

pub trait IntoObserver<R>
where
    R: for<'a> span::Registry<'a>,
{
    type Observer: Observe<R>;
    fn into_observer(self) -> Self::Observer;
}

/// An observer which does nothing.
pub struct NoObserver;

/// An observer which is an instance of one of two types that implement
/// `Observe`.
///
/// This is intended to be used when an observer implementation is chosen
/// conditionally, and the overhead of `Box<Observe>` is unwanted.
///
/// For example:
/// ```
/// # extern crate tokio_trace;
/// extern crate tokio_trace_subscriber;
/// use tokio_trace_subscriber::{observe, Event, Observe, SpanRef};
/// # use tokio_trace_subscriber::filter::{Filter, NoFilter};
/// # use tokio_trace::Span;
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
/// # fn observe_event<'a>(&self, _: &'a Event<'a>) {}
/// # fn enter(&self, _: &SpanRef) {}
/// # fn exit(&self, _: &SpanRef) {}
/// # fn close(&self, _: &SpanRef) {}
/// # fn filter(&self) -> &Filter { &NoFilter}
/// }
///
/// impl Observe for Bar {
///     // ...
/// # fn observe_event<'a>(&self, _: &'a Event<'a>) {}
/// # fn enter(&self, _: &SpanRef) {}
/// # fn exit(&self, _: &SpanRef) {}
/// # fn close(&self, _: &SpanRef) {}
/// # fn filter(&self) -> &Filter { &NoFilter}
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

// impl<O, F> Filter for WithFilter<O, F>
// where
//     O: Observe,
//     F: Filter,
// {
//     #[inline]
//     fn enabled(&self, metadata: &Metadata) -> bool {
//         self.filter.enabled(metadata) && self.inner.filter().enabled(metadata)
//     }

//     #[inline]
//     fn should_invalidate_filter(&self, metadata: &Metadata) -> bool {
//         self.filter.should_invalidate_filter(metadata)
//             || self.inner.filter().should_invalidate_filter(metadata)
//     }
// }

// impl<O, F> Observe for WithFilter<O, F>
// where
//     O: Observe,
//     F: Filter,
// {
//     #[inline]
//     fn observe_event<'a>(&self, event: &'a Event<'a>) {
//         self.inner.observe_event(event)
//     }

//     #[inline]
//     fn enter(&self, span: &SpanRef) {
//         self.inner.enter(span)
//     }

//     #[inline]
//     fn exit(&self, span: &SpanRef) {
//         self.inner.exit(span)
//     }

//     #[inline]
//     fn close(&self, span: &SpanRef) {
//         self.inner.close(span)
//     }

//     fn filter(&self) -> &Filter {
//         self
//     }
// }

// pub fn none() -> NoObserver {
//     NoObserver
// }

impl<T, R> ObserveExt<R> for T
where
    T: Observe<R>,
    R: for<'a> span::Registry<'a>,
{
}

impl<T, R> IntoObserver<R> for T
where
    T: Observe<R>,
    R: for<'a> span::Registry<'a>,
{
    type Observer = Self;
    fn into_observer(self) -> Self::Observer {
        self
    }
}

// XXX: maybe this should just be an impl of `Observe` for tuples of `(Observe, Observe)`...?
impl<A, B, R> Observe<R> for Tee<A, B>
where
    A: Observe<R>,
    B: Observe<R>,
    R: for<'a> span::Registry<'a>,
{
    fn event(&self, event: &Event, registry: &R) {
        self.a.event(event, registry);
        self.b.event(event, registry);
    }

    fn enter(&self, span: &span::Id, registry: &R) {
        self.a.enter(span, registry);
        self.b.enter(span, registry);
    }

    fn exit(&self, span: &span::Id, registry: &R) {
        self.a.exit(span, registry);
        self.b.exit(span, registry);
    }

    fn close(&self, span: &span::Id, registry: &R) {
        self.a.close(span, registry);
        self.b.close(span, registry);
    }

    // fn filter(&self) -> &Filter {
    //     self
    // }

    fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        match id {
            id if id == self.type_id() => Some(self as *const _ as *const ()),
            id if id == self.a.type_id() => Some(&self.a as *const _ as *const ()),
            id if id == self.b.type_id() => Some(&self.b as *const _ as *const ()),
            _ => None,
        }
    }
}

// impl<A, B> Filter for Tee<A, B>
// where
//     A: Observe,
//     B: Observe,
// {
//     fn enabled(&self, metadata: &Metadata) -> bool {
//         self.a.filter().enabled(metadata) || self.b.filter().enabled(metadata)
//     }

//     fn should_invalidate_filter(&self, metadata: &Metadata) -> bool {
//         self.a.filter().should_invalidate_filter(metadata)
//             || self.b.filter().should_invalidate_filter(metadata)
//     }
// }

// impl<A, B> Observe for Either<A, B>
// where
//     A: Observe,
//     B: Observe,
// {
//     fn observe_event<'a>(&self, event: &'a Event<'a>) {
//         match self {
//             Either::A(a) => a.observe_event(event),
//             Either::B(b) => b.observe_event(event),
//         }
//     }

//     fn enter(&self, span: &SpanRef) {
//         match self {
//             Either::A(a) => a.enter(span),
//             Either::B(b) => b.enter(span),
//         }
//     }

//     fn exit(&self, span: &SpanRef) {
//         match self {
//             Either::A(a) => a.exit(span),
//             Either::B(b) => b.exit(span),
//         }
//     }

//     fn close(&self, span: &SpanRef) {
//         match self {
//             Either::A(a) => a.close(span),
//             Either::B(b) => b.close(span),
//         }
//     }
// }

// impl<A, B> Filter for Either<A, B>
// where
//     A: Observe,
//     B: Observe,
// {
//     fn enabled(&self, metadata: &Metadata) -> bool {
//         match self {
//             Either::A(a) => a.filter().enabled(metadata),
//             Either::B(b) => b.filter().enabled(metadata),
//         }
//     }

//     fn should_invalidate_filter(&self, metadata: &Metadata) -> bool {
//         match self {
//             Either::A(a) => a.filter().should_invalidate_filter(metadata),
//             Either::B(b) => b.filter().should_invalidate_filter(metadata),
//         }
//     }
// }

// impl<S: SpanRef> Observe<S> for NoObserver {
//     fn event<'a>(&self, _event: &'a Event<'a>) {}

//     fn enter(&self, _span: &SpanRef) {}

//     fn exit(&self, _span: &SpanRef) {}

//     fn close(&self, _span: &SpanRef) {}

//     fn filter(&self) -> &Filter {
//         self
//     }
// }

// impl Filter for NoObserver {
//     fn enabled(&self, _metadata: &Metadata) -> bool {
//         false
//     }

//     fn should_invalidate_filter(&self, _metadata: &Metadata) -> bool {
//         false
//     }
// }
