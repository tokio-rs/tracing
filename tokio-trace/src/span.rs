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
//! of span exits and span closures. When execution exits a span,
//! [`exit`](::Subscriber::exit) will always be called with that span's ID to
//! notify the subscriber that the span has been exited. If the span has been
//! exited for the final time, the `exit` will be followed by a call to
//! [`close`](::Subscriber::close), signalling that the span has been closed.
//! Subscribers may expect that a span which has closed will not be entered
//! again.
//!
//! If there is only a single handle with the capacity to exit a span, dropping
//! that handle will automatically close the span, since the capacity to enter
//! it no longer exists. For example:
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
//! If one or more handles to a span exist, the span will be kept open until
//! that handle drops. However, a span may be explicitly asked to close by
//! calling the [`Span::close`] method. For example:
//! ```
//! # #[macro_use] extern crate tokio_trace;
//! # fn main() {
//! use tokio_trace::Span;
//!
//! let mut my_span = span!("my_span");
//! my_span.enter(|| {
//!     // Signal to my_span that it should close when it exits
//!     Span::current().close();
//! }); // --> Subscriber::exit(my_span); Subscriber::close(my_span)
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
//!
//! When a span is asked to close by explicitly calling `Span::close`, if it is
//! executing, it will wait until it exits to signal that it has been closed. If
//! it is not currently executing, it will signal closure immediately.
//!
//! Calls to `Span::close()` are *not* guaranteed to close the span immediately.
//! If multiple handles to the span exist, the span will not be closed until all
//! but the one which opened the span have been dropped. This is to ensure that
//! a subscriber never observes an inconsistant state; namely, a span being
//! entered after it has closed.
//!
//! ## Shared spans
//!
//! In general, it is rarely necessary to create multiple handles to a span that
//! persist outside of the period during which that span is executing. Users who
//! wish to have multiple handles capable of entering a span, or enter the same
//! span from multiple threads, may wish to use the [shared span] type instead.
//!
//! A `Shared` handle behaves similarly to a `Span` handle, but may be cloned
//! freely. Any `Shared` handle may be used to enter the span it corresponds to,
//! and (since `Shared` handles are `Send + Sync`) they may be used to allow
//! multiple threads to enter the same span.
//!
//! Shared handles may be created from a span using the [`IntoShared`] trait:
//! ```
//! # #[macro_use] extern crate tokio_trace;
//! # fn main() {
//! use tokio_trace::span::IntoShared;
//! // Convert a regular `Span` handle into a cloneable `Shared` handle.
//! let span = span!("foo").into_shared();
//!
//! // Entering a `Shared` span handle *consumes* the handle:
//! span.clone().enter(|| {
//!     // ...
//! });
//!
//! // When all `Shared` handles to the span have been consumed, it will close.
//! span.enter(|| {
//!     // ...
//! }) // --> Subscriber::close(span)
//! # }
//! ```
//!
//! Unlike `Span` handles, `Shared` spans are represented by `Arc` pointers, so
//! constructing them will allocate memory, if the span is enabled.
//!
//! **Note**: When _not_ using shared spans, it is possible to cause a span to
//! _never_ be dropped, by entering it, creating a second handle using
//! `Span::current`, and returning that handle from the closure which executes
//! inside the span, and then dropping that handle while the span is not
//! currently executing. Since the span is not currently executing, it has no
//! way to observe the second handle being dropped, and it cannot determine if
//! it is safe to close.  However, if all such handles are dropped while the
//! span is executing, the span will still be able to close normally. Spans will
//! only  fail to close in situations where the second handle is dropped while
//! the span is _not_ executing.
//!
//! This is possible because the logic for determining if a span can close is
//! intentionally cautious. Spans will only close if they _know_ they cannot be
//! entered; although a span failing to close does result in potentially
//! incorrect trace data, this is less serious of a problem than the
//! inconsistant state that arises when a span is closed and then re-entered.
//! The latter violates the assumptions that a reasonable subscriber
//! implementation could be expected to make.
//!
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
//! [`State`]: ::span::State
//! [`Attributes`]: ::span::Attributes
//! [shared span]: ::span::Shared
//! [`IntoShared`]: ::span::IntoShared
pub use tokio_trace_core::span::{Attributes, Event, Id, Span, SpanAttributes};

#[cfg(any(test, feature = "test-support"))]
pub use tokio_trace_core::span::{mock, MockSpan};

use std::{fmt, sync::Arc};
use tokio_trace_core::span::Enter;
use {
    field,
    subscriber::{self, RecordError},
};

/// Trait for converting a `Span` into a cloneable `Shared` span.
pub trait IntoShared {
    /// Returns a `Shared` span handle that can be cloned.
    ///
    /// This will allocate memory to store the span if the span is enabled.
    fn into_shared(self) -> Shared;
}

pub trait SpanExt: field::Record + ::sealed::Sealed {
    fn record<Q: ?Sized, V: ?Sized>(&mut self, field: &Q, value: &V) -> Result<(), RecordError>
    where
        Q: field::AsKey,
        V: field::Value;

    fn has_field_for<Q: ?Sized>(&self, field: &Q) -> bool
    where
        Q: field::AsKey;
}

impl IntoShared for Span {
    fn into_shared(self) -> Shared {
        Shared::from_span(self)
    }
}

/// A shared, heap-allocated span handle, which may be cloned.
///
/// Unlike a `Span` handle, entering a `Shared` span handle consumes the handle.
/// This is used to determine when the span may be closed. Thus, each `Shared`
/// handle may be thought of as representing a *single* attempt to enter the
/// span.
///
/// However, they may be cloned inexpensively (as they are internally
/// reference-counted).
// NOTE: It may also be worthwhile to provide an `UnsyncShared` type, that uses
// `Rc` rather than `Arc` for performance reasons? I'm not sure what the
// use-case for that would be, though.
#[derive(Clone, Debug)]
pub struct Shared {
    inner: Option<Arc<Enter>>,
}

impl Shared {
    /// Returns a `Shared` span handle that can be cloned.
    ///
    /// This function allocates memory to store the shared span, if the span is
    /// enabled.
    pub fn from_span(span: Span) -> Self {
        Self {
            inner: Enter::from_span(span).map(Arc::new),
        }
    }

    /// Executes the given function in the context of this span.
    ///
    /// Unlike `Span::enter`, this *consumes* the shared span handle.
    ///
    /// If this span is enabled, then this function enters the span, invokes
    /// and then exits the span. If the span is disabled, `f` will still be
    /// invoked, but in the context of the currently-executing span (if there is
    /// one).
    ///
    /// Returns the result of evaluating `f`.
    pub fn enter<F: FnOnce() -> T, T>(self, f: F) -> T {
        if let Some(inner) = self.inner {
            let guard = inner.enter();
            let result = f();
            inner.exit_and_join(guard);
            if Arc::strong_count(&inner) == 1 {
                inner.close();
            }
            result
        } else {
            f()
        }
    }

    /// Returns the `Id` of the parent of this span, if one exists.
    pub fn parent(&self) -> Option<Id> {
        self.inner.as_ref().and_then(|inner| inner.parent())
    }

    /// Returns the `Id` of the span, or `None` if it is disabled.
    pub fn id(&self) -> Option<Id> {
        self.inner.as_ref().map(|inner| inner.id())
    }

    /// Returns `true` if this span is enabled.
    pub fn is_enabled(&self) -> bool {
        self.inner.is_some()
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
    /// If this span is disabled, this function will do nothing. Otherwise, it
    /// returns `Ok(())` if the other span was added as a precedent of this
    /// span, or an error if this was not possible.
    pub fn follows_from(&self, from: Id) -> Result<(), subscriber::FollowsError> {
        self.inner
            .as_ref()
            .map(move |inner| inner.follows_from(from))
            .unwrap_or(Ok(()))
    }
}

impl ::sealed::Sealed for Span {}

impl SpanExt for Span {
    fn has_field_for<Q: ?Sized>(&self, field: &Q) -> bool
    where
        Q: field::AsKey,
    {
        self.metadata()
            .and_then(|meta| field.as_key(meta))
            .is_some()
    }

    fn record<Q: ?Sized, V: ?Sized>(&mut self, field: &Q, value: &V) -> Result<(), RecordError>
    where
        Q: field::AsKey,
        V: field::Value,
    {
        value.record(field, self)
    }
}

impl field::Record for Span {
    #[inline]
    fn record_i64<Q: ?Sized>(&mut self, field: &Q, value: i64) -> Result<(), RecordError>
    where
        Q: field::AsKey,
    {
        if let Some(meta) = self.metadata() {
            let key = field.as_key(meta).ok_or_else(RecordError::no_field)?;
            self.record_value_i64(&key, value)?;
        }
        Ok(())
    }

    #[inline]
    fn record_u64<Q: ?Sized>(&mut self, field: &Q, value: u64) -> Result<(), RecordError>
    where
        Q: field::AsKey,
    {
        if let Some(meta) = self.metadata() {
            let key = field.as_key(meta).ok_or_else(RecordError::no_field)?;
            self.record_value_u64(&key, value)?;
        }
        Ok(())
    }

    #[inline]
    fn record_bool<Q: ?Sized>(&mut self, field: &Q, value: bool) -> Result<(), RecordError>
    where
        Q: field::AsKey,
    {
        if let Some(meta) = self.metadata() {
            let key = field.as_key(meta).ok_or_else(RecordError::no_field)?;
            self.record_value_bool(&key, value)?;
        }
        Ok(())
    }

    #[inline]
    fn record_str<Q: ?Sized>(&mut self, field: &Q, value: &str) -> Result<(), RecordError>
    where
        Q: field::AsKey,
    {
        if let Some(meta) = self.metadata() {
            let key = field.as_key(meta).ok_or_else(RecordError::no_field)?;
            self.record_value_str(&key, value)?;
        }
        Ok(())
    }

    #[inline]
    fn record_fmt<Q: ?Sized>(&mut self, field: &Q, value: fmt::Arguments) -> Result<(), RecordError>
    where
        Q: field::AsKey,
    {
        if let Some(meta) = self.metadata() {
            let key = field.as_key(meta).ok_or_else(RecordError::no_field)?;
            self.record_value_fmt(&key, value)?;
        }
        Ok(())
    }
}

impl<'a> field::Record for Event<'a> {
    #[inline]
    fn record_i64<Q: ?Sized>(&mut self, field: &Q, value: i64) -> Result<(), RecordError>
    where
        Q: field::AsKey,
    {
        if let Some(meta) = self.metadata() {
            let key = field.as_key(meta).ok_or_else(RecordError::no_field)?;
            self.record_value_i64(&key, value)?;
        }
        Ok(())
    }

    #[inline]
    fn record_u64<Q: ?Sized>(&mut self, field: &Q, value: u64) -> Result<(), RecordError>
    where
        Q: field::AsKey,
    {
        if let Some(meta) = self.metadata() {
            let key = field.as_key(meta).ok_or_else(RecordError::no_field)?;
            self.record_value_u64(&key, value)?;
        }
        Ok(())
    }

    #[inline]
    fn record_bool<Q: ?Sized>(&mut self, field: &Q, value: bool) -> Result<(), RecordError>
    where
        Q: field::AsKey,
    {
        if let Some(meta) = self.metadata() {
            let key = field.as_key(meta).ok_or_else(RecordError::no_field)?;
            self.record_value_bool(&key, value)?;
        }
        Ok(())
    }

    #[inline]
    fn record_str<Q: ?Sized>(&mut self, field: &Q, value: &str) -> Result<(), RecordError>
    where
        Q: field::AsKey,
    {
        if let Some(meta) = self.metadata() {
            let key = field.as_key(meta).ok_or_else(RecordError::no_field)?;
            self.record_value_str(&key, value)?;
        }
        Ok(())
    }

    #[inline]
    fn record_fmt<Q: ?Sized>(&mut self, field: &Q, value: fmt::Arguments) -> Result<(), RecordError>
    where
        Q: field::AsKey,
    {
        if let Some(meta) = self.metadata() {
            let key = field.as_key(meta).ok_or_else(RecordError::no_field)?;
            self.record_value_fmt(&key, value)?;
        }
        Ok(())
    }
}

impl<'a> ::sealed::Sealed for Event<'a> {}

impl<'a> SpanExt for Event<'a> {
    fn has_field_for<Q: ?Sized>(&self, field: &Q) -> bool
    where
        Q: field::AsKey,
    {
        self.metadata()
            .and_then(|meta| field.as_key(meta))
            .is_some()
    }

    fn record<Q: ?Sized, V: ?Sized>(&mut self, field: &Q, value: &V) -> Result<(), RecordError>
    where
        Q: field::AsKey,
        V: field::Value,
    {
        value.record(field, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use {span, subscriber, Dispatch};

    #[test]
    fn exit_doesnt_finish_while_handles_still_exist() {
        // Test that exiting a span only marks it as "done" when no handles
        // that can re-enter the span exist.
        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("bar")))
            // The first time we exit "bar", there will be another handle with
            // which we could potentially re-enter bar.
            .exit(span::mock().named(Some("bar")))
            // Re-enter "bar", using the cloned handle.
            .enter(span::mock().named(Some("bar")))
            // Now, when we exit "bar", there is no handle to re-enter it, so
            // it should become "done".
            .exit(span::mock().named(Some("bar")))
            .close(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("foo")))
            .run();

        Dispatch::new(subscriber).as_default(|| {
            span!("foo").enter(|| {
                let mut bar = span!("bar",);
                let another_bar = bar.enter(|| {
                    // do nothing. exiting "bar" should leave it idle, since it can
                    // be re-entered.
                    let mut another_bar = Span::current();
                    another_bar.close();
                    another_bar
                });
                // Enter "bar" again. This time, the previously-requested
                // closure should be honored.
                bar.enter(move || {
                    // Drop the other handle to bar. Now, the span should be allowed
                    // to close.
                    drop(another_bar);
                });
            });
        });
    }

    #[test]
    fn handles_to_the_same_span_are_equal() {
        // Create a mock subscriber that will return `true` on calls to
        // `Subscriber::enabled`, so that the spans will be constructed. We
        // won't enter any spans in this test, so the subscriber won't actually
        // expect to see any spans.
        Dispatch::new(subscriber::mock().run()).as_default(|| {
            span!("foo").enter(|| {
                let foo1 = Span::current();
                let foo2 = Span::current();
                // Two handles that point to the same span are equal.
                assert_eq!(foo1, foo2);
            })
        });
    }

    #[test]
    fn handles_to_different_spans_are_not_equal() {
        Dispatch::new(subscriber::mock().run()).as_default(|| {
            // Even though these spans have the same name and fields, they will have
            // differing metadata, since they were created on different lines.
            let foo1 = span!("foo", bar = 1u64, baz = false);
            let foo2 = span!("foo", bar = 1u64, baz = false);

            assert_ne!(foo1, foo2);
            // assert_ne!(foo1.data(), foo2.data());
        });
    }

    #[test]
    fn handles_to_different_spans_with_the_same_metadata_are_not_equal() {
        // Every time time this function is called, it will return a _new
        // instance_ of a span with the same metadata, name, and fields.
        fn make_span() -> Span {
            span!("foo", bar = 1u64, baz = false)
        }

        Dispatch::new(subscriber::mock().run()).as_default(|| {
            let foo1 = make_span();
            let foo2 = make_span();

            assert_ne!(foo1, foo2);
            // assert_ne!(foo1.data(), foo2.data());
        });
    }

    #[test]
    fn spans_always_go_to_the_subscriber_that_tagged_them() {
        let subscriber1 = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .close(span::mock().named(Some("foo")))
            .done();
        let subscriber1 = Dispatch::new(subscriber1.run());
        let subscriber2 = Dispatch::new(subscriber::mock().run());

        let mut foo = subscriber1.as_default(|| {
            let mut foo = span!("foo");
            foo.enter(|| {});
            foo
        });
        // Even though we enter subscriber 2's context, the subscriber that
        // tagged the span should see the enter/exit.
        subscriber2.as_default(move || foo.enter(|| {}));
    }

    #[test]
    fn spans_always_go_to_the_subscriber_that_tagged_them_even_across_threads() {
        let subscriber1 = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .close(span::mock().named(Some("foo")))
            .done();
        let subscriber1 = Dispatch::new(subscriber1.run());
        let mut foo = subscriber1.as_default(|| {
            let mut foo = span!("foo");
            foo.enter(|| {});
            foo
        });

        // Even though we enter subscriber 2's context, the subscriber that
        // tagged the span should see the enter/exit.
        thread::spawn(move || {
            Dispatch::new(subscriber::mock().run()).as_default(|| {
                foo.enter(|| {});
            })
        }).join()
        .unwrap();
    }

    #[test]
    fn span_closes_on_drop() {
        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .close(span::mock().named(Some("foo")))
            .done()
            .run();
        Dispatch::new(subscriber).as_default(|| {
            let mut span = span!("foo");
            span.enter(|| {});
            drop(span);
        })
    }

    #[test]
    fn span_closes_after_event() {
        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .event()
            .exit(span::mock().named(Some("foo")))
            .close(span::mock().named(Some("foo")))
            .done()
            .run();
        Dispatch::new(subscriber).as_default(|| {
            span!("foo").enter(|| {
                Span::current().close();
                event!(::Level::Debug, {}, "my event!");
            });
        })
    }

    #[test]
    fn new_span_after_event() {
        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .event()
            .exit(span::mock().named(Some("foo")))
            .close(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("bar")))
            .exit(span::mock().named(Some("bar")))
            .close(span::mock().named(Some("bar")))
            .done()
            .run();
        Dispatch::new(subscriber).as_default(|| {
            span!("foo").enter(|| {
                Span::current().close();
                event!(::Level::Debug, {}, "my event!");
            });
            span!("bar").enter(|| {
                Span::current().close();
            });
        })
    }

    #[test]
    fn event_outside_of_span() {
        let subscriber = subscriber::mock()
            .event()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .close(span::mock().named(Some("foo")))
            .done()
            .run();
        Dispatch::new(subscriber).as_default(|| {
            event!(::Level::Debug, {}, "my event!");
            span!("foo").enter(|| {
                Span::current().close();
            });
        })
    }

    #[test]
    fn dropping_a_span_calls_drop_span() {
        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .close(span::mock().named(Some("foo")))
            .drop_span(span::mock().named(Some("foo")))
            .done()
            .run();
        Dispatch::new(subscriber).as_default(|| {
            let mut span = span!("foo");
            span.enter(|| {});
            drop(span);
        })
    }

    #[test]
    fn span_current_calls_clone_span() {
        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .clone_span(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .run();
        Dispatch::new(subscriber).as_default(|| {
            let mut span = span!("foo");
            let _span2 = span.enter(|| Span::current());
        })
    }

    #[test]
    fn drop_span_when_exiting_dispatchers_context() {
        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .clone_span(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .drop_span(span::mock().named(Some("foo")))
            .drop_span(span::mock().named(Some("foo")))
            .run();
        Dispatch::new(subscriber).as_default(|| {
            let mut span = span!("foo");
            let _span2 = span.enter(|| Span::current());
            drop(span);
        })
    }

    #[test]
    fn clone_and_drop_span_always_go_to_the_subscriber_that_tagged_the_span() {
        let subscriber1 = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .clone_span(span::mock().named(Some("foo")))
            .drop_span(span::mock().named(Some("foo")))
            .drop_span(span::mock().named(Some("foo")))
            .done();
        let subscriber1 = Dispatch::new(subscriber1.run());
        let subscriber2 = Dispatch::new(subscriber::mock().done().run());

        let mut foo = subscriber1.as_default(|| {
            let mut foo = span!("foo");
            foo.enter(|| {});
            foo
        });
        // Even though we enter subscriber 2's context, the subscriber that
        // tagged the span should see the enter/exit.
        subscriber2.as_default(move || {
            let foo2 = foo.enter(|| Span::current());
            drop(foo);
            drop(foo2);
        });
    }

    #[test]
    fn span_closes_when_idle() {
        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            // A second span is entered so that the mock subscriber will
            // expect "foo" at a separate point in time from when it is exited.
            .enter(span::mock().named(Some("bar")))
            .close(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("bar")))
            .close(span::mock().named(Some("bar")))
            .done()
            .run();
        Dispatch::new(subscriber).as_default(|| {
            let mut foo = span!("foo");
            foo.enter(|| {});

            span!("bar").enter(|| {
                // Since `foo` is not executing, it should close immediately.
                foo.close();
            });

            assert!(foo.is_closed());
        })
    }

    #[test]
    fn entering_a_closed_span_is_a_no_op() {
        let subscriber = subscriber::mock()
            .enter(span::mock().named(Some("foo")))
            .exit(span::mock().named(Some("foo")))
            .close(span::mock().named(Some("foo")))
            .done()
            .run();
        Dispatch::new(subscriber).as_default(|| {
            let mut foo = span!("foo");
            foo.enter(|| {});

            foo.close();

            foo.enter(|| {
                // The subscriber expects nothing else to happen after the first
                // exit.
            });
            assert!(foo.is_closed());
        })
    }

    #[test]
    fn span_doesnt_close_if_it_never_opened() {
        let subscriber = subscriber::mock().done().run();
        Dispatch::new(subscriber).as_default(|| {
            let span = span!("foo");
            drop(span);
        })
    }

    mod shared {
        use super::*;

        #[test]
        fn span_closes_on_drop() {
            let subscriber = subscriber::mock()
                .enter(span::mock().named(Some("foo")))
                .exit(span::mock().named(Some("foo")))
                .close(span::mock().named(Some("foo")))
                .done()
                .run();
            Dispatch::new(subscriber).as_default(|| span!("foo").into_shared().enter(|| {}))
        }

        #[test]
        fn span_doesnt_close_if_it_never_opened() {
            let subscriber = subscriber::mock().done().run();
            Dispatch::new(subscriber).as_default(|| {
                let span = span!("foo").into_shared();
                drop(span);
            })
        }

        #[test]
        fn shared_only_calls_drop_span_when_the_last_handle_is_dropped() {
            let subscriber = subscriber::mock()
                .enter(span::mock().named(Some("foo")))
                .exit(span::mock().named(Some("foo")))
                .close(span::mock().named(Some("foo")))
                .drop_span(span::mock().named(Some("foo")))
                .done()
                .run();
            Dispatch::new(subscriber).as_default(|| {
                let span = span!("foo").into_shared();
                let foo1 = span.clone();
                let foo2 = span.clone();
                drop(foo1);
                drop(span);
                foo2.enter(|| {})
            })
        }

        #[test]
        fn exit_doesnt_finish_concurrently_executing_spans() {
            // Test that exiting a span only marks it as "done" when no other
            // threads are still executing inside that span.
            use std::sync::{Arc, Barrier};

            let subscriber = subscriber::mock()
                .enter(span::mock().named(Some("baz")))
                // Main thread enters "quux".
                .enter(span::mock().named(Some("quux")))
                // Spawned thread also enters "quux".
                .enter(span::mock().named(Some("quux")))
                // When the main thread exits "quux", it will still be running in the
                // spawned thread.
                .exit(span::mock().named(Some("quux")))
                // Now, when this thread exits "quux", there is no handle to re-enter it, so
                // it should become "done".
                .exit(span::mock().named(Some("quux")))
                .close(span::mock().named(Some("quux")))
                // "baz" never had more than one handle, so it should also become
                // "done" when we exit it.
                .exit(span::mock().named(Some("baz")))
                .close(span::mock().named(Some("baz")))
                .done()
                .run();

            Dispatch::new(subscriber).as_default(|| {
                let barrier1 = Arc::new(Barrier::new(2));
                let barrier2 = Arc::new(Barrier::new(2));
                // Make copies of the barriers for thread 2 to wait on.
                let t2_barrier1 = barrier1.clone();
                let t2_barrier2 = barrier2.clone();

                span!("baz",).enter(move || {
                    let quux = span!("quux",).into_shared();
                    let quux2 = quux.clone();
                    let handle = thread::Builder::new()
                        .name("thread-2".to_string())
                        .spawn(move || {
                            quux2.enter(|| {
                                // Once this thread has entered "quux", allow thread 1
                                // to exit.
                                t2_barrier1.wait();
                                // Wait for the main thread to allow us to exit.
                                t2_barrier2.wait();
                            })
                        }).expect("spawn test thread");
                    quux.enter(|| {
                        // Wait for thread 2 to enter "quux". When we exit "quux", it
                        // should stay running, since it's running in the other thread.
                        barrier1.wait();
                    });
                    // After we exit "quux", wait for the second barrier, so the other
                    // thread unblocks and exits "quux".
                    barrier2.wait();
                    handle.join().unwrap();
                });
            });
        }

        #[test]
        fn exit_doesnt_finish_while_handles_still_exist() {
            // Test that exiting a span only marks it as "done" when no handles
            // that can re-enter the span exist.
            let subscriber = subscriber::mock()
                .enter(span::mock().named(Some("foo")))
                .enter(span::mock().named(Some("bar")))
                // The first time we exit "bar", there will be another handle with
                // which we could potentially re-enter bar.
                .exit(span::mock().named(Some("bar")))
                // Re-enter "bar", using the cloned handle.
                .enter(span::mock().named(Some("bar")))
                // Now, when we exit "bar", there is no handle to re-enter it, so
                // it should become "done".
                .exit(span::mock().named(Some("bar")))
                .close(span::mock().named(Some("bar")))
                .exit(span::mock().named(Some("foo")))
                .done()
                .run();

            Dispatch::new(subscriber).as_default(|| {
                span!("foo").enter(|| {
                    let bar = span!("bar").into_shared();
                    bar.clone().enter(|| {
                        // do nothing. exiting "bar" should leave it idle, since it can
                        // be re-entered.
                    });
                    // Enter "bar" again. consuming the last handle to `bar`
                    // close the span automatically.
                    bar.enter(|| {});
                });
            });
        }
    }
}
