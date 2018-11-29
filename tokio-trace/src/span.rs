//! Spans represent periods of time in the execution of a program.
//!
//! # Entering a Span
//!
//! A thread of execution is said to _enter_ a span when it begins executing,
//! and _exit_ the span when it switches to another context. Spans may be
//! entered through the [`enter`](`Span::enter`) method, which enters the target span,
//! performs a given function (either a closure or a function pointer), exits
//! the span, and then returns the result.
//!
//! Calling `enter` on a span handle enters the span that handle corresponds to,
//! if the span exists:
//! ```
//! # #[macro_use] extern crate tokio_trace;
//! # fn main() {
//! let my_var: u64 = 5;
//! let mut my_span = span!("my_span", my_var = &my_var);
//!
//! my_span.enter(|| {
//!     // perform some work in the context of `my_span`...
//! });
//!
//! // Perform some work outside of the context of `my_span`...
//!
//! my_span.enter(|| {
//!     // Perform some more work in the context of `my_span`.
//! });
//! # }
//! ```
//!
//! # The Span Lifecycle
//!
//! Execution may enter and exit a span multiple times before that
//! span is _closed_. Consider, for example, a future which has an associated
//! span and enters that span every time it is polled:
//! ```rust
//! # extern crate tokio_trace;
//! # extern crate futures;
//! # use futures::{Future, Poll, Async};
//! struct MyFuture {
//!    // data
//!    span: tokio_trace::Span,
//! }
//!
//! impl Future for MyFuture {
//!     type Item = ();
//!     type Error = ();
//!
//!     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//!         self.span.enter(|| {
//!             // Do actual future work
//! # Ok(Async::Ready(()))
//!         })
//!     }
//! }
//! ```
//!
//! If this future was spawned on an executor, it might yield one or more times
//! before `poll` returns `Ok(Async::Ready)`. If the future were to yield, then
//! the executor would move on to poll the next future, which may _also_ enter
//! an associated span or series of spans. Therefore, it is valid for a span to
//! be entered repeatedly before it completes. Only the time when that span or
//! one of its children was the current span is considered to be time spent in
//! that span. A span which is not executing and has not yet been closed is said
//! to be _idle_.
//!
//! Because spans may be entered and exited multiple times before they close,
//! [`Subscriber`]s have separate trait methods which are called to notify them
//! of span exits and when span handles are dropped. When execution exits a
//! span, [`exit`](::Subscriber::exit) will always be called with that span's ID
//! to notify the subscriber that the span has been exited. When span handles
//! are dropped, the [`drop_span`](::Subscriber::drop_span) method is called
//! with that span's ID. The subscriber may use this to determine whether or not
//! the span will be entered again.
//!
//! If there is only a single handle with the capacity to exit a span, dropping
//! that handle "close" the span, since the capacity to enter it no longer
//! exists. For example:
//! ```
//! # #[macro_use] extern crate tokio_trace;
//! # fn main() {
//! {
//!     span!("my_span").enter(|| {
//!         // perform some work in the context of `my_span`...
//!     }); // --> Subscriber::exit(my_span)
//!
//!     // The handle to `my_span` only lives inside of this block; when it is
//!     // dropped, the subscriber will be informed that `my_span` has closed.
//!
//! } // --> Subscriber::close(my_span)
//! # }
//! ```
//!
//! A span may be explicitly closed before when the span handle is dropped by
//! calling the [`Span::close`] method. Doing so will drop that handle the next
//! time it is exited. For example:
//! ```
//! # #[macro_use] extern crate tokio_trace;
//! # fn main() {
//! use tokio_trace::Span;
//!
//! let mut my_span = span!("my_span");
//! // Signal to my_span that it should close when it exits
//! my_span.close();
//! my_span.enter(|| {
//!    // ...
//! }); // --> Subscriber::exit(my_span); Subscriber::drop_span(my_span)
//!
//! // The handle to `my_span` still exists, but it now knows that the span was
//! // closed while it was executing.
//! my_span.is_closed(); // ==> true
//!
//! // Attempting to enter the span using the handle again will do nothing.
//! my_span.enter(|| {
//!     // no-op
//! });
//! # }
//! ```
//! However, if multiple handles exist, the span can still be re-entered even if
//! one or more is dropped. For determining when _all_ handles to a span have
//! been dropped, `Subscriber`s have a [`clone_span`](::Subscriber::clone_span)
//! method, which is called every time a span handle is cloned. Combined with
//! `drop_span`, this may be used to track the number of handles to a given span
//! --- if `drop_span` has been called one more time than the number of calls to
//! `clone_span` for a given ID, then no more handles to the span with that ID
//! exist. The subscriber may then treat it as closed.
//!
//! # Accessing a Span's Attributes
//!
//! The [`Attributes`] type represents a *non-entering* reference to a `Span`'s data
//! --- a set of key-value pairs (known as _fields_), a creation timestamp,
//! a reference to the span's parent in the trace tree, and metadata describing
//! the source code location where the span was created. This data is provided
//! to the [`Subscriber`] when the span is created; it may then choose to cache
//! the data for future use, record it in some manner, or discard it completely.
//!
//! [`Subscriber`]: ::Subscriber
// TODO: remove this re-export?
pub use tokio_trace_core::span::Span as Id;

#[cfg(any(test, feature = "test-support"))]
pub use tokio_trace_core::span::{mock, MockSpan};

use std::{
    borrow::Borrow,
    cmp, fmt,
    hash::{Hash, Hasher},
};
use {
    dispatcher::{self, Dispatch},
    field,
    subscriber::{Interest, Subscriber},
    Meta,
};

/// A handle representing a span, with the capability to enter the span if it
/// exists.
///
/// If the span was rejected by the current `Subscriber`'s filter, entering the
/// span will silently do nothing. Thus, the handle can be used in the same
/// manner regardless of whether or not the trace is currently being collected.
#[derive(Clone, PartialEq, Hash)]
pub struct Span {
    /// A handle used to enter the span when it is not executing.
    ///
    /// If this is `None`, then the span has either closed or was never enabled.
    inner: Option<Enter>,

    /// Set to `true` when the span closes.
    ///
    /// This allows us to distinguish if `inner` is `None` because the span was
    /// never enabled (and thus the inner state was never created), or if the
    /// previously entered, but it is now closed.
    is_closed: bool,
}

/// `Event`s represent single points in time where something occurred during the
/// execution of a program.
///
/// An event can be compared to a log record in unstructured logging, but with
/// two key differences:
/// - Events exist _within the context of a [`Span`]_. Unlike log lines, they may
///   be located within the trace tree, allowing visibility into the context in
///   which the event occurred.
/// - Events have structured key-value data known as _fields_, as well as a
///   textual message. In general, a majority of the data associated with an
///   event should be in the event's fields rather than in the textual message,
///   as the fields are more structed.
///
/// [`Span`]: ::span::Span
#[derive(PartialEq, Hash)]
pub struct Event<'a> {
    /// A handle used to enter the span when it is not executing.
    ///
    /// If this is `None`, then the span has either closed or was never enabled.
    inner: Option<Inner<'a>>,
}

/// A handle representing the capacity to enter a span which is known to exist.
///
/// Unlike `Span`, this type is only constructed for spans which _have_ been
/// enabled by the current filter. This type is primarily used for implementing
/// span handles; users should typically not need to interact with it directly.
#[derive(Debug)]
pub(crate) struct Inner<'a> {
    /// The span's ID, as provided by `subscriber`.
    id: Id,

    /// The subscriber that will receive events relating to this span.
    ///
    /// This should be the same subscriber that provided this span with its
    /// `id`.
    subscriber: Dispatch,

    /// A flag indicating that the span has been instructed to close when
    /// possible.
    closed: bool,

    meta: &'a Meta<'a>,
}

/// When an `Inner` corresponds to a `Span` rather than an `Event`, it can be
/// used to enter that span.
type Enter = Inner<'static>;

/// A guard representing a span which has been entered and is currently
/// executing.
///
/// This guard may be used to exit the span, returning an `Enter` to
/// re-enter it.
///
/// This type is primarily used for implementing span handles; users should
/// typically not need to interact with it directly.
#[derive(Debug)]
#[must_use = "once a span has been entered, it should be exited"]
struct Entered {
    inner: Enter,
}

// ===== impl Span =====

impl Span {
    /// Constructs a new `Span` originating from the given [`Callsite`].
    ///
    /// The new span will be constructed by the currently-active [`Subscriber`],
    /// with the [current span] as its parent (if one exists).
    ///
    /// If the new span is enabled, then the provided function `if_enabled` is
    /// envoked on it before it is returned. This allows [field values] and/or
    /// [`follows_from` annotations] to be added to the span, but skips this
    /// work for spans which are disabled.
    ///
    /// [`Callsite`]: ::callsite::Callsite
    /// [`Subscriber`]: ::subscriber::Subscriber
    /// [current span]: ::span::Span::current
    /// [field values]: ::span::Span::record
    /// [`follows_from` annotations]: ::span::Span::follows_from
    #[inline]
    pub fn new<F>(interest: Interest, meta: &'static Meta<'static>, if_enabled: F) -> Span
    where
        F: FnOnce(&mut Span),
    {
        if interest.is_never() {
            return Span::new_disabled();
        }
        let mut span = dispatcher::with_current(|dispatch| {
            if interest.is_sometimes() && !dispatch.enabled(meta) {
                return Span {
                    inner: None,
                    is_closed: false,
                };
            }
            let id = dispatch.new_static(meta);
            let inner = Some(Enter::new(id, dispatch, meta));
            Self {
                inner,
                is_closed: false,
            }
        });
        if !span.is_disabled() {
            if_enabled(&mut span);
        }
        span
    }

    /// Constructs a new disabled span.
    pub fn new_disabled() -> Span {
        Span {
            inner: None,
            is_closed: false,
        }
    }

    /// Executes the given function in the context of this span.
    ///
    /// If this span is enabled, then this function enters the span, invokes
    /// and then exits the span. If the span is disabled, `f` will still be
    /// invoked, but in the context of the currently-executing span (if there is
    /// one).
    ///
    /// Returns the result of evaluating `f`.
    pub fn enter<F: FnOnce() -> T, T>(&mut self, f: F) -> T {
        match self.inner.take() {
            Some(inner) => dispatcher::with_default(inner.subscriber.clone(), || {
                let guard = inner.enter();
                let result = f();
                self.inner = guard.exit();
                result
            }),
            None => f(),
        }
    }

    /// Returns a [`Key`](::field::Key) for the field with the given `name`, if
    /// one exists,
    pub fn key_for<Q>(&self, name: &Q) -> Option<field::Key>
    where
        Q: Borrow<str>,
    {
        self.inner
            .as_ref()
            .and_then(|inner| inner.meta.fields().key_for(name))
    }

    /// Returns true if this `Span` has a field for the given
    /// [`Key`](::field::Key) or field name.
    pub fn has_field_for<Q: ?Sized>(&self, field: &Q) -> bool
    where
        Q: field::AsKey,
    {
        self.metadata()
            .and_then(|meta| field.as_key(meta))
            .is_some()
    }

    /// Records that the field described by `field` has the value `value`.
    pub fn record<Q: ?Sized, V: ?Sized>(&mut self, field: &Q, value: &V) -> &mut Self
    where
        Q: field::AsKey,
        V: field::Value,
    {
        if let Some(ref mut inner) = self.inner {
            value.record(field, inner);
        }
        self
    }

    /// Signals that this span should close the next time it is exited, or when
    /// it is dropped.
    pub fn close(&mut self) {
        if let Some(ref mut inner) = self.inner {
            inner.close();
        }
        self.is_closed = true;
    }

    /// Returns `true` if this span is closed.
    pub fn is_closed(&self) -> bool {
        self.is_closed
    }

    /// Returns `true` if this span was disabled by the subscriber and does not
    /// exist.
    pub fn is_disabled(&self) -> bool {
        self.inner.is_none() && !self.is_closed
    }

    /// Indicates that the span with the given ID has an indirect causal
    /// relationship with this span.
    ///
    /// This relationship differs somewhat from the parent-child relationship: a
    /// span may have any number of prior spans, rather than a single one; and
    /// spans are not considered to be executing _inside_ of the spans they
    /// follow from. This means that a span may close even if subsequent spans
    /// that follow from it are still open, and time spent inside of a
    /// subsequent span should not be included in the time its precedents were
    /// executing. This is used to model causal relationships such as when a
    /// single future spawns several related background tasks, et cetera.
    ///
    /// If this span is disabled, or the resulting follows-from relationship
    /// would be invalid, this function will do nothing.
    pub fn follows_from(&self, from: Id) -> &Self {
        if let Some(ref inner) = self.inner {
            inner.follows_from(from);
        }
        self
    }

    /// Returns this span's `Id`, if it is enabled.
    pub fn id(&self) -> Option<Id> {
        self.inner.as_ref().map(Enter::id)
    }

    /// Returns this span's `Meta`, if it is enabled.
    pub fn metadata(&self) -> Option<&'static Meta<'static>> {
        self.inner.as_ref().map(|inner| inner.metadata())
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut span = f.debug_struct("Span");
        if let Some(ref inner) = self.inner {
            span.field("id", &inner.id())
        } else {
            span.field("disabled", &true)
        }.finish()
    }
}

// ===== impl Event =====

impl<'a> Event<'a> {
    /// Constructs a new `Span` originating from the given [`Callsite`].
    ///
    /// The new span will be constructed by the currently-active [`Subscriber`],
    /// with the [current span] as its parent (if one exists).
    ///
    /// If the new span is enabled, then the provided function `if_enabled` is
    /// envoked on it before it is returned. This allows [field values] and/or
    /// [`follows_from` annotations] to be added to the span, but skips this
    /// work for spans which are disabled.
    ///
    /// [`Callsite`]: ::callsite::Callsite
    /// [`Subscriber`]: ::subscriber::Subscriber
    /// [current span]: ::span::Span::current
    /// [field values]: ::span::Span::record
    /// [`follows_from` annotations]: ::span::Span::follows_from
    #[inline]
    pub fn new<F>(interest: Interest, meta: &'a Meta<'a>, if_enabled: F) -> Self
    where
        F: FnOnce(&mut Self),
    {
        if interest.is_never() {
            return Self { inner: None };
        }
        let mut event = dispatcher::with_current(|dispatch| {
            if interest.is_sometimes() && !dispatch.enabled(meta) {
                return Self { inner: None };
            }
            let id = dispatch.new_span(meta);
            let inner = Inner::new(id, dispatch, meta);
            Self { inner: Some(inner) }
        });
        if !event.is_disabled() {
            if_enabled(&mut event);
        }
        event
    }

    /// Adds a formattable message describing the event that occurred.
    pub fn message(&mut self, key: &field::Key, message: fmt::Arguments) -> &mut Self {
        if let Some(ref mut inner) = self.inner {
            inner.subscriber.record_fmt(&inner.id, key, message);
        }
        self
    }

    /// Returns a [`Key`](::field::Key) for the field with the given `name`, if
    /// one exists,
    pub fn key_for<Q>(&self, name: &Q) -> Option<field::Key>
    where
        Q: Borrow<str>,
    {
        self.inner
            .as_ref()
            .and_then(|inner| inner.meta.fields().key_for(name))
    }

    /// Returns true if this `Event` has a field for the given
    /// [`Key`](::field::Key) or field name.
    pub fn has_field<Q: ?Sized>(&self, field: &Q) -> bool
    where
        Q: field::AsKey,
    {
        self.metadata()
            .and_then(|meta| field.as_key(meta))
            .is_some()
    }

    /// Records that the field described by `field` has the value `value`.
    pub fn record<Q: ?Sized, V: ?Sized>(&mut self, field: &Q, value: &V) -> &mut Self
    where
        Q: field::AsKey,
        V: field::Value,
    {
        if let Some(ref mut inner) = self.inner {
            value.record(field, inner);
        }
        self
    }

    /// Returns `true` if this span was disabled by the subscriber and does not
    /// exist.
    pub fn is_disabled(&self) -> bool {
        self.inner.is_none()
    }

    /// Indicates that the span with the given ID has an indirect causal
    /// relationship with this event.
    ///
    /// This relationship differs somewhat from the parent-child relationship: a
    /// span may have any number of prior spans, rather than a single one; and
    /// spans are not considered to be executing _inside_ of the spans they
    /// follow from. This means that a span may close even if subsequent spans
    /// that follow from it are still open, and time spent inside of a
    /// subsequent span should not be included in the time its precedents were
    /// executing. This is used to model causal relationships such as when a
    /// single future spawns several related background tasks, et cetera.
    ///
    /// If this event is disabled, or the resulting follows-from relationship
    /// would be invalid, this function will do nothing.
    pub fn follows_from(&self, from: Id) -> &Self {
        if let Some(ref inner) = self.inner {
            inner.follows_from(from);
        }
        self
    }

    /// Returns this span's `Id`, if it is enabled.
    pub fn id(&self) -> Option<Id> {
        self.inner.as_ref().map(Enter::id)
    }

    /// Returns this span's `Meta`, if it is enabled.
    pub fn metadata(&self) -> Option<&'a Meta<'a>> {
        self.inner.as_ref().map(|inner| inner.metadata())
    }
}

// ===== impl Inner =====

impl<'a> Inner<'a> {
    /// Indicates that the span with the given ID has an indirect causal
    /// relationship with this span.
    ///
    /// This relationship differs somewhat from the parent-child relationship: a
    /// span may have any number of prior spans, rather than a single one; and
    /// spans are not considered to be executing _inside_ of the spans they
    /// follow from. This means that a span may close even if subsequent spans
    /// that follow from it are still open, and time spent inside of a
    /// subsequent span should not be included in the time its precedents were
    /// executing. This is used to model causal relationships such as when a
    /// single future spawns several related background tasks, et cetera.
    ///
    /// If this span is disabled, this function will do nothing. Otherwise, it
    /// returns `Ok(())` if the other span was added as a precedent of this
    /// span, or an error if this was not possible.
    fn follows_from(&self, from: Id) {
        self.subscriber.add_follows_from(&self.id, from)
    }

    /// Returns the span's ID.
    fn id(&self) -> Id {
        self.id.clone()
    }

    /// Returns the span's metadata.
    fn metadata(&self) -> &'a Meta<'a> {
        self.meta
    }

    /// Record a signed 64-bit integer value.
    fn record_value_i64(&self, field: &field::Key, value: i64) {
        self.subscriber.record_i64(&self.id, field, value)
    }

    /// Record an unsigned 64-bit integer value.
    fn record_value_u64(&self, field: &field::Key, value: u64) {
        self.subscriber.record_u64(&self.id, field, value)
    }

    /// Record a boolean value.
    fn record_value_bool(&self, field: &field::Key, value: bool) {
        self.subscriber.record_bool(&self.id, field, value)
    }

    /// Record a string value.
    fn record_value_str(&self, field: &field::Key, value: &str) {
        self.subscriber.record_str(&self.id, field, value)
    }

    /// Record a precompiled set of format arguments value.
    fn record_value_fmt(&self, field: &field::Key, value: fmt::Arguments) {
        self.subscriber.record_fmt(&self.id, field, value)
    }

    fn new(id: Id, subscriber: &Dispatch, meta: &'a Meta<'a>) -> Self {
        Self {
            id,
            subscriber: subscriber.clone(),
            closed: false,
            meta,
        }
    }
}

impl<'a> cmp::PartialEq for Inner<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<'a> Hash for Inner<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<'a> Drop for Inner<'a> {
    fn drop(&mut self) {
        self.subscriber.drop_span(self.id.clone());
    }
}

impl<'a> Clone for Inner<'a> {
    fn clone(&self) -> Self {
        Self {
            id: self.subscriber.clone_span(&self.id),
            subscriber: self.subscriber.clone(),
            closed: self.closed,
            meta: self.meta,
        }
    }
}

impl<'a> field::Record for Inner<'a> {
    #[inline]
    fn record_i64<Q: ?Sized>(&mut self, field: &Q, value: i64)
    where
        Q: field::AsKey,
    {
        if let Some(key) = field.as_key(self.metadata()) {
            self.record_value_i64(&key, value);
        }
    }

    #[inline]
    fn record_u64<Q: ?Sized>(&mut self, field: &Q, value: u64)
    where
        Q: field::AsKey,
    {
        if let Some(key) = field.as_key(self.metadata()) {
            self.record_value_u64(&key, value);
        }
    }

    #[inline]
    fn record_bool<Q: ?Sized>(&mut self, field: &Q, value: bool)
    where
        Q: field::AsKey,
    {
        if let Some(key) = field.as_key(self.metadata()) {
            self.record_value_bool(&key, value);
        }
    }

    #[inline]
    fn record_str<Q: ?Sized>(&mut self, field: &Q, value: &str)
    where
        Q: field::AsKey,
    {
        if let Some(key) = field.as_key(self.metadata()) {
            self.record_value_str(&key, value);
        }
    }

    #[inline]
    fn record_fmt<Q: ?Sized>(&mut self, field: &Q, value: fmt::Arguments)
    where
        Q: field::AsKey,
    {
        if let Some(key) = field.as_key(self.metadata()) {
            self.record_value_fmt(&key, value);
        }
    }
}

// ===== impl Enter =====

impl Enter {
    /// Indicates that this handle will not be reused to enter the span again.
    ///
    /// After calling `close`, the `Entered` guard returned by `self.enter()`
    /// will _drop_ this handle when it is exited.
    fn close(&mut self) {
        self.closed = true;
    }

    /// Enters the span, returning a guard that may be used to exit the span and
    /// re-enter the prior span.
    ///
    /// This is used internally to implement `Span::enter`. It may be used for
    /// writing custom span handles, but should generally not be called directly
    /// when entering a span.
    fn enter(self) -> Entered {
        self.subscriber.enter(&self.id);
        Entered { inner: self }
    }
}

// ===== impl Entered =====
impl Entered {
    /// Exit the `Entered` guard, returning an `Enter` handle that may be used
    /// to re-enter the span, or `None` if the span closed while performing the
    /// exit.
    fn exit(self) -> Option<Enter> {
        self.inner.subscriber.exit(&self.inner.id);
        if self.inner.closed {
            // Dropping `inner` will allow it to perform the closure if
            // able.
            None
        } else {
            Some(self.inner)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use {span, subscriber, Dispatch};

    #[test]
    fn closed_handle_dropped_when_used() {
        // Test that exiting a span only marks it as "done" when no handles
        // that can re-enter the span exist.
        let subscriber = subscriber::mock()
            .enter(span::mock().named("foo"))
            .drop_span(span::mock().named("bar"))
            .enter(span::mock().named("bar"))
            .exit(span::mock().named("bar"))
            .drop_span(span::mock().named("bar"))
            .exit(span::mock().named("foo"))
            .run();

        dispatcher::with_default(Dispatch::new(subscriber), || {
            span!("foo").enter(|| {
                let bar = span!("bar");
                let mut another_bar = bar.clone();
                drop(bar);

                another_bar.close();
                another_bar.enter(|| {});
                // After we exit `another_bar`, it should close and not be
                // re-entered.
                another_bar.enter(|| {});
            });
        });
    }

    #[test]
    fn handles_to_the_same_span_are_equal() {
        // Create a mock subscriber that will return `true` on calls to
        // `Subscriber::enabled`, so that the spans will be constructed. We
        // won't enter any spans in this test, so the subscriber won't actually
        // expect to see any spans.
        dispatcher::with_default(Dispatch::new(subscriber::mock().run()), || {
            let foo1 = span!("foo");
            let foo2 = foo1.clone();
            // Two handles that point to the same span are equal.
            assert_eq!(foo1, foo2);
        });
    }

    #[test]
    fn handles_to_different_spans_are_not_equal() {
        dispatcher::with_default(Dispatch::new(subscriber::mock().run()), || {
            // Even though these spans have the same name and fields, they will have
            // differing metadata, since they were created on different lines.
            let foo1 = span!("foo", bar = 1u64, baz = false);
            let foo2 = span!("foo", bar = 1u64, baz = false);

            assert_ne!(foo1, foo2);
        });
    }

    #[test]
    fn handles_to_different_spans_with_the_same_metadata_are_not_equal() {
        // Every time time this function is called, it will return a _new
        // instance_ of a span with the same metadata, name, and fields.
        fn make_span() -> Span {
            span!("foo", bar = 1u64, baz = false)
        }

        dispatcher::with_default(Dispatch::new(subscriber::mock().run()), || {
            let foo1 = make_span();
            let foo2 = make_span();

            assert_ne!(foo1, foo2);
            // assert_ne!(foo1.data(), foo2.data());
        });
    }

    #[test]
    fn spans_always_go_to_the_subscriber_that_tagged_them() {
        let subscriber1 = subscriber::mock()
            .enter(span::mock().named("foo"))
            .exit(span::mock().named("foo"))
            .enter(span::mock().named("foo"))
            .exit(span::mock().named("foo"))
            .drop_span(span::mock().named("foo"))
            .done();
        let subscriber1 = Dispatch::new(subscriber1.run());
        let subscriber2 = Dispatch::new(subscriber::mock().run());

        let mut foo = dispatcher::with_default(subscriber1, || {
            let mut foo = span!("foo");
            foo.enter(|| {});
            foo
        });
        // Even though we enter subscriber 2's context, the subscriber that
        // tagged the span should see the enter/exit.
        dispatcher::with_default(subscriber2, move || foo.enter(|| {}));
    }

    #[test]
    fn spans_always_go_to_the_subscriber_that_tagged_them_even_across_threads() {
        let subscriber1 = subscriber::mock()
            .enter(span::mock().named("foo"))
            .exit(span::mock().named("foo"))
            .enter(span::mock().named("foo"))
            .exit(span::mock().named("foo"))
            .drop_span(span::mock().named("foo"))
            .done();
        let subscriber1 = Dispatch::new(subscriber1.run());
        let mut foo = dispatcher::with_default(subscriber1, || {
            let mut foo = span!("foo");
            foo.enter(|| {});
            foo
        });

        // Even though we enter subscriber 2's context, the subscriber that
        // tagged the span should see the enter/exit.
        thread::spawn(move || {
            dispatcher::with_default(Dispatch::new(subscriber::mock().run()), || {
                foo.enter(|| {});
            })
        }).join()
        .unwrap();
    }

    #[test]
    fn dropping_a_span_calls_drop_span() {
        let (subscriber, handle) = subscriber::mock()
            .enter(span::mock().named("foo"))
            .exit(span::mock().named("foo"))
            .drop_span(span::mock().named("foo"))
            .done()
            .run_with_handle();
        dispatcher::with_default(Dispatch::new(subscriber), || {
            let mut span = span!("foo");
            span.enter(|| {});
            drop(span);
        });

        handle.assert_finished();
    }

    #[test]
    fn span_closes_after_event() {
        let (subscriber, handle) = subscriber::mock()
            .enter(span::mock().named("foo"))
            .event()
            .exit(span::mock().named("foo"))
            .drop_span(span::mock().named("foo"))
            .done()
            .run_with_handle();
        dispatcher::with_default(Dispatch::new(subscriber), || {
            span!("foo").enter(|| {
                event!(::Level::DEBUG, {}, "my event!");
            });
        });

        handle.assert_finished();
    }

    #[test]
    fn new_span_after_event() {
        let (subscriber, handle) = subscriber::mock()
            .enter(span::mock().named("foo"))
            .event()
            .exit(span::mock().named("foo"))
            .drop_span(span::mock().named("foo"))
            .enter(span::mock().named("bar"))
            .exit(span::mock().named("bar"))
            .drop_span(span::mock().named("bar"))
            .done()
            .run_with_handle();
        dispatcher::with_default(Dispatch::new(subscriber), || {
            span!("foo").enter(|| {
                event!(::Level::DEBUG, {}, "my event!");
            });
            span!("bar").enter(|| {});
        });

        handle.assert_finished();
    }

    #[test]
    fn event_outside_of_span() {
        let (subscriber, handle) = subscriber::mock()
            .event()
            .enter(span::mock().named("foo"))
            .exit(span::mock().named("foo"))
            .drop_span(span::mock().named("foo"))
            .done()
            .run_with_handle();
        dispatcher::with_default(Dispatch::new(subscriber), || {
            debug!("my event!");
            span!("foo").enter(|| {});
        });

        handle.assert_finished();
    }

    #[test]
    fn cloning_a_span_calls_clone_span() {
        let (subscriber, handle) = subscriber::mock()
            .clone_span(span::mock().named("foo"))
            .run_with_handle();
        dispatcher::with_default(Dispatch::new(subscriber), || {
            let span = span!("foo");
            let _span2 = span.clone();
        });

        handle.assert_finished();
    }

    #[test]
    fn drop_span_when_exiting_dispatchers_context() {
        let (subscriber, handle) = subscriber::mock()
            .clone_span(span::mock().named("foo"))
            .drop_span(span::mock().named("foo"))
            .drop_span(span::mock().named("foo"))
            .run_with_handle();
        dispatcher::with_default(Dispatch::new(subscriber), || {
            let span = span!("foo");
            let _span2 = span.clone();
            drop(span);
        });

        handle.assert_finished();
    }

    #[test]
    fn clone_and_drop_span_always_go_to_the_subscriber_that_tagged_the_span() {
        let (subscriber1, handle1) = subscriber::mock()
            .enter(span::mock().named("foo"))
            .exit(span::mock().named("foo"))
            .clone_span(span::mock().named("foo"))
            .enter(span::mock().named("foo"))
            .exit(span::mock().named("foo"))
            .drop_span(span::mock().named("foo"))
            .drop_span(span::mock().named("foo"))
            .run_with_handle();
        let subscriber1 = Dispatch::new(subscriber1);
        let subscriber2 = Dispatch::new(subscriber::mock().done().run());

        let mut foo = dispatcher::with_default(subscriber1, || {
            let mut foo = span!("foo");
            foo.enter(|| {});
            foo
        });
        // Even though we enter subscriber 2's context, the subscriber that
        // tagged the span should see the enter/exit.
        dispatcher::with_default(subscriber2, move || {
            let foo2 = foo.clone();
            foo.enter(|| {});
            drop(foo);
            drop(foo2);
        });

        handle1.assert_finished();
    }

    #[test]
    fn span_closes_when_exited() {
        let (subscriber, handle) = subscriber::mock()
            .enter(span::mock().named("foo"))
            .exit(span::mock().named("foo"))
            .enter(span::mock().named("foo"))
            .exit(span::mock().named("foo"))
            .drop_span(span::mock().named("foo"))
            .done()
            .run_with_handle();
        dispatcher::with_default(Dispatch::new(subscriber), || {
            let mut foo = span!("foo");
            assert!(!foo.is_closed());

            foo.enter(|| {});
            assert!(!foo.is_closed());

            foo.close();
            assert!(foo.is_closed());

            // Now that `foo` has closed, entering it should do nothing.
            foo.enter(|| {});
            assert!(foo.is_closed());
        });

        handle.assert_finished();
    }

    #[test]
    fn entering_a_closed_span_again_is_a_no_op() {
        let (subscriber, handle) = subscriber::mock()
            .enter(span::mock().named("foo"))
            .exit(span::mock().named("foo"))
            .drop_span(span::mock().named("foo"))
            .done()
            .run_with_handle();
        dispatcher::with_default(Dispatch::new(subscriber), || {
            let mut foo = span!("foo");
            foo.close();

            foo.enter(|| {
                // When we exit `foo` this time, it will close, and entering it
                // again will do nothing.
            });

            foo.enter(|| {
                // The subscriber expects nothing else to happen after the first
                // exit.
            });
            assert!(foo.is_closed());
        });

        handle.assert_finished();
    }
}
