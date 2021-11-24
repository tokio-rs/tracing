//! Spans represent periods of time in which a program was executing in a
//! particular context.
//!
//! A span consists of [fields], user-defined key-value pairs of arbitrary data
//! that describe the context the span represents, and a set of fixed attributes
//! that describe all `tracing` spans and events. Attributes describing spans
//! include:
//!
//! - An [`Id`] assigned by the subscriber that uniquely identifies it in relation
//!   to other spans.
//! - The span's [parent] in the trace tree.
//! - [Metadata] that describes static characteristics of all spans
//!   originating from that callsite, such as its name, source code location,
//!   [verbosity level], and the names of its fields.
//!
//! # Creating Spans
//!
//! Spans are created using the [`span!`] macro. This macro is invoked with the
//! following arguments, in order:
//!
//! - The [`target`] and/or [`parent`][parent] attributes, if the user wishes to
//!   override their default values.
//! - The span's [verbosity level]
//! - A string literal providing the span's name.
//! - Finally, between zero and 32 arbitrary key/value fields.
//!
//! [`target`]: super::Metadata::target()
//!
//! For example:
//! ```rust
//! use tracing::{span, Level};
//!
//! /// Construct a new span at the `INFO` level named "my_span", with a single
//! /// field named answer , with the value `42`.
//! let my_span = span!(Level::INFO, "my_span", answer = 42);
//! ```
//!
//! The documentation for the [`span!`] macro provides additional examples of
//! the various options that exist when creating spans.
//!
//! The [`trace_span!`], [`debug_span!`], [`info_span!`], [`warn_span!`], and
//! [`error_span!`] exist as shorthand for constructing spans at various
//! verbosity levels.
//!
//! ## Recording Span Creation
//!
//! The [`Attributes`] type contains data associated with a span, and is
//! provided to the [collector] when a new span is created. It contains
//! the span's metadata, the ID of [the span's parent][parent] if one was
//! explicitly set, and any fields whose values were recorded when the span was
//! constructed. The collector, which is responsible for recording `tracing`
//! data, can then store or record these values.
//!
//! # The Span Lifecycle
//!
//! ## Entering a Span
//!
//! A thread of execution is said to _enter_ a span when it begins executing,
//! and _exit_ the span when it switches to another context. Spans may be
//! entered through the [`enter`] and [`in_scope`] methods.
//!
//! The `enter` method enters a span, returning a [guard] that exits the span
//! when dropped
//! ```
//! # use tracing::{Level, span};
//! let my_var: u64 = 5;
//! let my_span = span!(Level::TRACE, "my_span", my_var);
//!
//! // `my_span` exists but has not been entered.
//!
//! // Enter `my_span`...
//! let _enter = my_span.enter();
//!
//! // Perform some work inside of the context of `my_span`...
//! // Dropping the `_enter` guard will exit the span.
//!```
//!
//! <div class="example-wrap" style="display:inline-block"><pre class="compile_fail" style="white-space:normal;font:inherit;">
//!
//!  **Warning**: In asynchronous code that uses async/await syntax,
//!  [`Span::enter`] may produce incorrect traces if the returned drop
//!  guard is held across an await point. See
//!  [the method documentation][Span#in-asynchronous-code] for details.
//!
//! </pre></div>
//!
//! `in_scope` takes a closure or function pointer and executes it inside the
//! span.
//! ```
//! # use tracing::{Level, span};
//! let my_var: u64 = 5;
//! let my_span = span!(Level::TRACE, "my_span", my_var = &my_var);
//!
//! my_span.in_scope(|| {
//!     // perform some work in the context of `my_span`...
//! });
//!
//! // Perform some work outside of the context of `my_span`...
//!
//! my_span.in_scope(|| {
//!     // Perform some more work in the context of `my_span`.
//! });
//! ```
//!
//! <div class="example-wrap" style="display:inline-block">
//! <pre class="ignore" style="white-space:normal;font:inherit;">
//! <strong>Note</strong>: Since entering a span takes <code>&self</code>, and
//! <code>Span</code>s are <code>Clone</code>, <code>Send</code>, and
//! <code>Sync</code>, it is entirely valid for multiple threads to enter the
//! same span concurrently.
//! </pre></div>
//!
//! ## Span Relationships
//!
//! Spans form a tree structure — unless it is a root span, all spans have a
//! _parent_, and may have one or more _children_. When a new span is created,
//! the current span becomes the new span's parent. The total execution time of
//! a span consists of the time spent in that span and in the entire subtree
//! represented by its children. Thus, a parent span always lasts for at least
//! as long as the longest-executing span in its subtree.
//!
//! ```
//! # use tracing::{Level, span};
//! // this span is considered the "root" of a new trace tree:
//! span!(Level::INFO, "root").in_scope(|| {
//!     // since we are now inside "root", this span is considered a child
//!     // of "root":
//!     span!(Level::DEBUG, "outer_child").in_scope(|| {
//!         // this span is a child of "outer_child", which is in turn a
//!         // child of "root":
//!         span!(Level::TRACE, "inner_child").in_scope(|| {
//!             // and so on...
//!         });
//!     });
//!     // another span created here would also be a child of "root".
//! });
//!```
//!
//! In addition, the parent of a span may be explicitly specified in
//! the `span!` macro. For example:
//!
//! ```rust
//! # use tracing::{Level, span};
//! // Create, but do not enter, a span called "foo".
//! let foo = span!(Level::INFO, "foo");
//!
//! // Create and enter a span called "bar".
//! let bar = span!(Level::INFO, "bar");
//! let _enter = bar.enter();
//!
//! // Although we have currently entered "bar", "baz"'s parent span
//! // will be "foo".
//! let baz = span!(parent: &foo, Level::INFO, "baz");
//! ```
//!
//! A child span should typically be considered _part_ of its parent. For
//! example, if a collector is recording the length of time spent in various
//! spans, it should generally include the time spent in a span's children as
//! part of that span's duration.
//!
//! In addition to having zero or one parent, a span may also _follow from_ any
//! number of other spans. This indicates a causal relationship between the span
//! and the spans that it follows from, but a follower is *not* typically
//! considered part of the duration of the span it follows. Unlike the parent, a
//! span may record that it follows from another span after it is created, using
//! the [`follows_from`] method.
//!
//! As an example, consider a listener task in a server. As the listener accepts
//! incoming connections, it spawns new tasks that handle those connections. We
//! might want to have a span representing the listener, and instrument each
//! spawned handler task with its own span. We would want our instrumentation to
//! record that the handler tasks were spawned as a result of the listener task.
//! However, we might not consider the handler tasks to be _part_ of the time
//! spent in the listener task, so we would not consider those spans children of
//! the listener span. Instead, we would record that the handler tasks follow
//! from the listener, recording the causal relationship but treating the spans
//! as separate durations.
//!
//! ## Closing Spans
//!
//! Execution may enter and exit a span multiple times before that span is
//! _closed_. Consider, for example, a future which has an associated
//! span and enters that span every time it is polled:
//! ```rust
//! # use futures::{Future, Poll, Async};
//! struct MyFuture {
//!    // data
//!    span: tracing::Span,
//! }
//!
//! impl Future for MyFuture {
//!     type Item = ();
//!     type Error = ();
//!
//!     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//!         let _enter = self.span.enter();
//!         // Do actual future work...
//! # Ok(Async::Ready(()))
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
//! [collector]s have separate trait methods which are called to notify them
//! of span exits and when span handles are dropped. When execution exits a
//! span, [`exit`] will always be called with that span's ID to notify the
//! collector that the span has been exited. When span handles are dropped, the
//! [`drop_span`] method is called with that span's ID. The collector may use
//! this to determine whether or not the span will be entered again.
//!
//! If there is only a single handle with the capacity to exit a span, dropping
//! that handle "closes" the span, since the capacity to enter it no longer
//! exists. For example:
//! ```
//! # use tracing::{Level, span};
//! {
//!     span!(Level::TRACE, "my_span").in_scope(|| {
//!         // perform some work in the context of `my_span`...
//!     }); // --> Collect::exit(my_span)
//!
//!     // The handle to `my_span` only lives inside of this block; when it is
//!     // dropped, the collector will be informed via `drop_span`.
//!
//! } // --> Collect::drop_span(my_span)
//! ```
//!
//! However, if multiple handles exist, the span can still be re-entered even if
//! one or more is dropped. For determining when _all_ handles to a span have
//! been dropped, collectors have a [`clone_span`] method, which is called
//! every time a span handle is cloned. Combined with `drop_span`, this may be
//! used to track the number of handles to a given span — if `drop_span` has
//! been called one more time than the number of calls to `clone_span` for a
//! given ID, then no more handles to the span with that ID exist. The
//! collector may then treat it as closed.
//!
//! # When to use spans
//!
//! As a rule of thumb, spans should be used to represent discrete units of work
//! (e.g., a given request's lifetime in a server) or periods of time spent in a
//! given context (e.g., time spent interacting with an instance of an external
//! system, such as a database).
//!
//! Which scopes in a program correspond to new spans depend somewhat on user
//! intent. For example, consider the case of a loop in a program. Should we
//! construct one span and perform the entire loop inside of that span, like:
//!
//! ```rust
//! # use tracing::{Level, span};
//! # let n = 1;
//! let span = span!(Level::TRACE, "my_loop");
//! let _enter = span.enter();
//! for i in 0..n {
//!     # let _ = i;
//!     // ...
//! }
//! ```
//! Or, should we create a new span for each iteration of the loop, as in:
//! ```rust
//! # use tracing::{Level, span};
//! # let n = 1u64;
//! for i in 0..n {
//!     let span = span!(Level::TRACE, "my_loop", iteration = i);
//!     let _enter = span.enter();
//!     // ...
//! }
//! ```
//!
//! Depending on the circumstances, we might want to do either, or both. For
//! example, if we want to know how long was spent in the loop overall, we would
//! create a single span around the entire loop; whereas if we wanted to know how
//! much time was spent in each individual iteration, we would enter a new span
//! on every iteration.
//!
//! [fields]: super::field
//! [Metadata]: super::Metadata
//! [verbosity level]: super::Level
//! [`span!`]: super::span!
//! [`trace_span!`]: super::trace_span!
//! [`debug_span!`]: super::debug_span!
//! [`info_span!`]: super::info_span!
//! [`warn_span!`]: super::warn_span!
//! [`error_span!`]: super::error_span!
//! [`clone_span`]: super::collect::Collect::clone_span()
//! [`drop_span`]: super::collect::Collect::drop_span()
//! [`exit`]: super::collect::Collect::exit
//! [collector]: super::collect::Collect
//! [`enter`]: Span::enter()
//! [`in_scope`]: Span::in_scope()
//! [`follows_from`]: Span::follows_from()
//! [guard]: Entered
//! [parent]: #span-relationships
pub use tracing_core::span::{Attributes, Id, Record};

use crate::{
    dispatch::{self, Dispatch},
    field, Metadata,
};
use core::{
    cmp, fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem,
    ops::Deref,
};

/// Trait implemented by types which have a span `Id`.
pub trait AsId: crate::sealed::Sealed {
    /// Returns the `Id` of the span that `self` corresponds to, or `None` if
    /// this corresponds to a disabled span.
    fn as_id(&self) -> Option<&Id>;
}

/// A handle representing a span, with the capability to enter the span if it
/// exists.
///
/// If the span was rejected by the current `Collector`'s filter, entering the
/// span will silently do nothing. Thus, the handle can be used in the same
/// manner regardless of whether or not the trace is currently being collected.
#[derive(Clone)]
pub struct Span {
    /// A handle used to enter the span when it is not executing.
    ///
    /// If this is `None`, then the span has either closed or was never enabled.
    inner: Option<Inner>,
    /// Metadata describing the span.
    ///
    /// This might be `Some` even if `inner` is `None`, in the case that the
    /// span is disabled but the metadata is needed for `log` support.
    meta: Option<&'static Metadata<'static>>,
}

/// A handle representing the capacity to enter a span which is known to exist.
///
/// Unlike `Span`, this type is only constructed for spans which _have_ been
/// enabled by the current filter. This type is primarily used for implementing
/// span handles; users should typically not need to interact with it directly.
#[derive(Debug)]
pub(crate) struct Inner {
    /// The span's ID, as provided by `collector`.
    id: Id,

    /// The collector that will receive events relating to this span.
    ///
    /// This should be the same collector that provided this span with its
    /// `id`.
    collector: Dispatch,
}

/// A guard representing a span which has been entered and is currently
/// executing.
///
/// When the guard is dropped, the span will be exited.
///
/// This is returned by the [`Span::enter`] function.
///
/// [`Span::enter`]: super::Span::enter()
#[derive(Debug)]
#[must_use = "once a span has been entered, it should be exited"]
pub struct Entered<'a> {
    span: &'a Span,

    /// ```compile_fail
    /// use tracing::span::*;
    /// trait AssertSend: Send {}
    ///
    /// impl AssertSend for Entered<'_> {}
    /// ```
    _not_send: PhantomNotSend,
}

/// An owned version of [`Entered`], a guard representing a span which has been
/// entered and is currently executing.
///
/// When the guard is dropped, the span will be exited.
///
/// This is returned by the [`Span::entered`] function.
///
/// [`Span::entered`]: super::Span::entered()
#[derive(Debug)]
#[must_use = "once a span has been entered, it should be exited"]
pub struct EnteredSpan {
    span: Span,

    /// ```compile_fail
    /// use tracing::span::*;
    /// trait AssertSend: Send {}
    ///
    /// impl AssertSend for EnteredSpan {}
    /// ```
    _not_send: PhantomNotSend,
}

/// `log` target for all span lifecycle (creation/enter/exit/close) records.
#[cfg(feature = "log")]
const LIFECYCLE_LOG_TARGET: &str = "tracing::span";
/// `log` target for span activity (enter/exit) records.
#[cfg(feature = "log")]
const ACTIVITY_LOG_TARGET: &str = "tracing::span::active";

// ===== impl Span =====

impl Span {
    /// Constructs a new `Span` with the given [metadata] and set of
    /// [field values].
    ///
    /// The new span will be constructed by the currently-active [collector],
    /// with the current span as its parent (if one exists).
    ///
    /// After the span is constructed, [field values] and/or [`follows_from`]
    /// annotations may be added to it.
    ///
    /// [metadata]: mod@super::metadata
    /// [collector]: super::collect::Collect
    /// [field values]: super::field::ValueSet
    /// [`follows_from`]: super::Span::follows_from()
    pub fn new(meta: &'static Metadata<'static>, values: &field::ValueSet<'_>) -> Span {
        dispatch::get_default(|dispatch| Self::new_with(meta, values, dispatch))
    }

    #[inline]
    #[doc(hidden)]
    pub fn new_with(
        meta: &'static Metadata<'static>,
        values: &field::ValueSet<'_>,
        dispatch: &Dispatch,
    ) -> Span {
        let new_span = Attributes::new(meta, values);
        Self::make_with(meta, new_span, dispatch)
    }

    /// Constructs a new `Span` as the root of its own trace tree, with the
    /// given [metadata] and set of [field values].
    ///
    /// After the span is constructed, [field values] and/or [`follows_from`]
    /// annotations may be added to it.
    ///
    /// [metadata]: mod@super::metadata
    /// [field values]: super::field::ValueSet
    /// [`follows_from`]: super::Span::follows_from()
    pub fn new_root(meta: &'static Metadata<'static>, values: &field::ValueSet<'_>) -> Span {
        dispatch::get_default(|dispatch| Self::new_root_with(meta, values, dispatch))
    }

    #[inline]
    #[doc(hidden)]
    pub fn new_root_with(
        meta: &'static Metadata<'static>,
        values: &field::ValueSet<'_>,
        dispatch: &Dispatch,
    ) -> Span {
        let new_span = Attributes::new_root(meta, values);
        Self::make_with(meta, new_span, dispatch)
    }

    /// Constructs a new `Span` as child of the given parent span, with the
    /// given [metadata] and set of [field values].
    ///
    /// After the span is constructed, [field values] and/or [`follows_from`]
    /// annotations may be added to it.
    ///
    /// [metadata]: mod@super::metadata
    /// [field values]: super::field::ValueSet
    /// [`follows_from`]: super::Span::follows_from()
    pub fn child_of(
        parent: impl Into<Option<Id>>,
        meta: &'static Metadata<'static>,
        values: &field::ValueSet<'_>,
    ) -> Span {
        let mut parent = parent.into();
        dispatch::get_default(move |dispatch| {
            Self::child_of_with(Option::take(&mut parent), meta, values, dispatch)
        })
    }

    #[inline]
    #[doc(hidden)]
    pub fn child_of_with(
        parent: impl Into<Option<Id>>,
        meta: &'static Metadata<'static>,
        values: &field::ValueSet<'_>,
        dispatch: &Dispatch,
    ) -> Span {
        let new_span = match parent.into() {
            Some(parent) => Attributes::child_of(parent, meta, values),
            None => Attributes::new_root(meta, values),
        };
        Self::make_with(meta, new_span, dispatch)
    }

    /// Constructs a new disabled span with the given `Metadata`.
    ///
    /// This should be used when a span is constructed from a known callsite,
    /// but the collector indicates that it is disabled.
    ///
    /// Entering, exiting, and recording values on this span will not notify the
    /// `Collector` but _may_ record log messages if the `log` feature flag is
    /// enabled.
    #[inline(always)]
    pub fn new_disabled(meta: &'static Metadata<'static>) -> Span {
        Self {
            inner: None,
            meta: Some(meta),
        }
    }

    /// Constructs a new span that is *completely disabled*.
    ///
    /// This can be used rather than `Option<Span>` to represent cases where a
    /// span is not present.
    ///
    /// Entering, exiting, and recording values on this span will do nothing.
    #[inline(always)]
    pub const fn none() -> Span {
        Self {
            inner: None,
            meta: None,
        }
    }

    /// Returns a handle to the span [considered by the `Collector`] to be the
    /// current span.
    ///
    /// If the collector indicates that it does not track the current span, or
    /// that the thread from which this function is called is not currently
    /// inside a span, the returned span will be disabled.
    ///
    /// [considered by the `Collector`]: super::collect::Collect::current_span()
    pub fn current() -> Span {
        dispatch::get_default(|dispatch| {
            if let Some((id, meta)) = dispatch.current_span().into_inner() {
                let id = dispatch.clone_span(&id);
                Self {
                    inner: Some(Inner::new(id, dispatch)),
                    meta: Some(meta),
                }
            } else {
                Self::none()
            }
        })
    }

    fn make_with(
        meta: &'static Metadata<'static>,
        new_span: Attributes<'_>,
        dispatch: &Dispatch,
    ) -> Span {
        let attrs = &new_span;
        let id = dispatch.new_span(attrs);
        let inner = Some(Inner::new(id, dispatch));

        let span = Self {
            inner,
            meta: Some(meta),
        };

        if_log_enabled! { *meta.level(), {
            let target = if attrs.is_empty() {
                LIFECYCLE_LOG_TARGET
            } else {
                meta.target()
            };
            span.log(target, level_to_log!(*meta.level()), format_args!("++ {}{}", meta.name(), FmtAttrs(attrs)));
        }}

        span
    }

    /// Enters this span, returning a guard that will exit the span when dropped.
    ///
    /// If this span is enabled by the current collector, then this function will
    /// call [`Collect::enter`] with the span's [`Id`], and dropping the guard
    /// will call [`Collect::exit`]. If the span is disabled, this does
    /// nothing.
    ///
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    ///
    /// **Note**: The returned [`Entered`] guard does not
    /// implement `Send`. Dropping the guard will exit *this* span,
    /// and if the guard is sent to another thread and dropped there, that thread may
    /// never have entered this span. Thus, `Entered` should not be sent
    /// between threads.
    ///
    /// </pre></div>
    ///
    /// **Warning**: in asynchronous code that uses [async/await syntax][syntax],
    /// [`Span::enter`] should be used very carefully or avoided entirely. Holding
    /// the drop guard returned by `Span::enter` across `.await` points will
    /// result in incorrect traces. For example,
    ///
    /// ```
    /// # use tracing::info_span;
    /// # async fn some_other_async_function() {}
    /// async fn my_async_function() {
    ///     let span = info_span!("my_async_function");
    ///
    ///     // THIS WILL RESULT IN INCORRECT TRACES
    ///     let _enter = span.enter();
    ///     some_other_async_function().await;
    ///
    ///     // ...
    /// }
    /// ```
    ///
    /// The drop guard returned by `Span::enter` exits the span when it is
    /// dropped. When an async function or async block yields at an `.await`
    /// point, the current scope is _exited_, but values in that scope are
    /// **not** dropped (because the async block will eventually resume
    /// execution from that await point). This means that _another_ task will
    /// begin executing while _remaining_ in the entered span. This results in
    /// an incorrect trace.
    ///
    /// Instead of using `Span::enter` in asynchronous code, prefer the
    /// following:
    ///
    /// * To enter a span for a synchronous section of code within an async
    ///   block or function, prefer [`Span::in_scope`]. Since `in_scope` takes a
    ///   synchronous closure and exits the span when the closure returns, the
    ///   span will always be exited before the next await point. For example:
    ///   ```
    ///   # use tracing::info_span;
    ///   # async fn some_other_async_function(_: ()) {}
    ///   async fn my_async_function() {
    ///       let span = info_span!("my_async_function");
    ///
    ///       let some_value = span.in_scope(|| {
    ///           // run some synchronous code inside the span...
    ///       });
    ///
    ///       // This is okay! The span has already been exited before we reach
    ///       // the await point.
    ///       some_other_async_function(some_value).await;
    ///
    ///       // ...
    ///   }
    ///   ```
    /// * For instrumenting asynchronous code, `tracing` provides the
    ///   [`Future::instrument` combinator][instrument] for
    ///   attaching a span to a future (async function or block). This will
    ///   enter the span _every_ time the future is polled, and exit it whenever
    ///   the future yields.
    ///
    ///   `Instrument` can be used with an async block inside an async function:
    ///   ```ignore
    ///   # use tracing::info_span;
    ///   use tracing::Instrument;
    ///
    ///   # async fn some_other_async_function() {}
    ///   async fn my_async_function() {
    ///       let span = info_span!("my_async_function");
    ///       async move {
    ///          // This is correct! If we yield here, the span will be exited,
    ///          // and re-entered when we resume.
    ///          some_other_async_function().await;
    ///
    ///          //more asynchronous code inside the span...
    ///
    ///       }
    ///         // instrument the async block with the span...
    ///         .instrument(span)
    ///         // ...and await it.
    ///         .await
    ///   }
    ///   ```
    ///
    ///   It can also be used to instrument calls to async functions at the
    ///   callsite:
    ///   ```ignore
    ///   # use tracing::debug_span;
    ///   use tracing::Instrument;
    ///
    ///   # async fn some_other_async_function() {}
    ///   async fn my_async_function() {
    ///       let some_value = some_other_async_function()
    ///          .instrument(debug_span!("some_other_async_function"))
    ///          .await;
    ///
    ///       // ...
    ///   }
    ///   ```
    ///
    /// * The [`#[instrument]` attribute macro][attr] can automatically generate
    ///   correct code when used on an async function:
    ///
    ///   ```ignore
    ///   # async fn some_other_async_function() {}
    ///   #[tracing::instrument(level = "info")]
    ///   async fn my_async_function() {
    ///
    ///       // This is correct! If we yield here, the span will be exited,
    ///       // and re-entered when we resume.
    ///       some_other_async_function().await;
    ///
    ///       // ...
    ///
    ///   }
    ///   ```
    ///
    /// [syntax]: https://rust-lang.github.io/async-book/01_getting_started/04_async_await_primer.html
    /// [instrument]: crate::Instrument
    /// [attr]: macro@crate::instrument
    ///
    /// # Examples
    ///
    /// ```
    /// # use tracing::{span, Level};
    /// let span = span!(Level::INFO, "my_span");
    /// let guard = span.enter();
    ///
    /// // code here is within the span
    ///
    /// drop(guard);
    ///
    /// // code here is no longer within the span
    ///
    /// ```
    ///
    /// Guards need not be explicitly dropped:
    ///
    /// ```
    /// # use tracing::trace_span;
    /// fn my_function() -> String {
    ///     // enter a span for the duration of this function.
    ///     let span = trace_span!("my_function");
    ///     let _enter = span.enter();
    ///
    ///     // anything happening in functions we call is still inside the span...
    ///     my_other_function();
    ///
    ///     // returning from the function drops the guard, exiting the span.
    ///     return "Hello world".to_owned();
    /// }
    ///
    /// fn my_other_function() {
    ///     // ...
    /// }
    /// ```
    ///
    /// Sub-scopes may be created to limit the duration for which the span is
    /// entered:
    ///
    /// ```
    /// # use tracing::{info, info_span};
    /// let span = info_span!("my_great_span");
    ///
    /// {
    ///     let _enter = span.enter();
    ///
    ///     // this event occurs inside the span.
    ///     info!("i'm in the span!");
    ///
    ///     // exiting the scope drops the guard, exiting the span.
    /// }
    ///
    /// // this event is not inside the span.
    /// info!("i'm outside the span!")
    /// ```
    ///
    /// [`Collect::enter`]: super::collect::Collect::enter()
    /// [`Collect::exit`]: super::collect::Collect::exit()
    /// [`Id`]: super::Id
    #[inline]
    pub fn enter(&self) -> Entered<'_> {
        self.do_enter();
        Entered {
            span: self,
            _not_send: PhantomNotSend,
        }
    }

    /// Enters this span, consuming it and returning a [guard][`EnteredSpan`]
    /// that will exit the span when dropped.
    ///
    /// If this span is enabled by the current collector, then this function will
    /// call [`Collect::enter`] with the span's [`Id`], and dropping the guard
    /// will call [`Collect::exit`]. If the span is disabled, this does
    /// nothing.
    ///
    /// This is similar to the [`Span::enter`] method, except that it moves the
    /// span by value into the returned guard, rather than borrowing it.
    /// Therefore, this method can be used to create and enter a span in a
    /// single expression, without requiring a `let`-binding. For example:
    ///
    /// ```
    /// # use tracing::info_span;
    /// let _span = info_span!("something_interesting").entered();
    /// ```
    /// rather than:
    /// ```
    /// # use tracing::info_span;
    /// let span = info_span!("something_interesting");
    /// let _e = span.enter();
    /// ```
    ///
    /// Furthermore, `entered` may be used when the span must be stored in some
    /// other struct or be passed to a function while remaining entered.
    ///
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    ///
    /// **Note**: The returned [`EnteredSpan`] guard does not
    /// implement `Send`. Dropping the guard will exit *this* span,
    /// and if the guard is sent to another thread and dropped there, that thread may
    /// never have entered this span. Thus, `EnteredSpan`s should not be sent
    /// between threads.
    ///
    /// </pre></div>
    ///
    /// **Warning**: in asynchronous code that uses [async/await syntax][syntax],
    /// [`Span::entered`] should be used very carefully or avoided entirely. Holding
    /// the drop guard returned by `Span::entered` across `.await` points will
    /// result in incorrect traces. See the documentation for the
    /// [`Span::enter`] method for details.
    ///
    /// [syntax]: https://rust-lang.github.io/async-book/01_getting_started/04_async_await_primer.html
    ///
    /// # Examples
    ///
    /// The returned guard can be [explicitly exited][EnteredSpan::exit],
    /// returning the un-entered span:
    ///
    /// ```
    /// # use tracing::{Level, span};
    /// let span = span!(Level::INFO, "doing_something").entered();
    ///
    /// // code here is within the span
    ///
    /// // explicitly exit the span, returning it
    /// let span = span.exit();
    ///
    /// // code here is no longer within the span
    ///
    /// // enter the span again
    /// let span = span.entered();
    ///
    /// // now we are inside the span once again
    /// ```
    ///
    /// Guards need not be explicitly dropped:
    ///
    /// ```
    /// # use tracing::trace_span;
    /// fn my_function() -> String {
    ///     // enter a span for the duration of this function.
    ///     let span = trace_span!("my_function").entered();
    ///
    ///     // anything happening in functions we call is still inside the span...
    ///     my_other_function();
    ///
    ///     // returning from the function drops the guard, exiting the span.
    ///     return "Hello world".to_owned();
    /// }
    ///
    /// fn my_other_function() {
    ///     // ...
    /// }
    /// ```
    ///
    /// Since the [`EnteredSpan`] guard can dereference to the [`Span`] itself,
    /// the span may still be accessed while entered. For example:
    ///
    /// ```rust
    /// # use tracing::info_span;
    /// use tracing::field;
    ///
    /// // create the span with an empty field, and enter it.
    /// let span = info_span!("my_span", some_field = field::Empty).entered();
    ///
    /// // we can still record a value for the field while the span is entered.
    /// span.record("some_field", &"hello world!");
    /// ```
    ///
    /// [`Collect::enter`]: super::collect::Collect::enter()
    /// [`Collect::exit`]: super::collect::Collect::exit()
    /// [`Id`]: super::Id
    #[inline]
    pub fn entered(self) -> EnteredSpan {
        self.do_enter();
        EnteredSpan {
            span: self,
            _not_send: PhantomNotSend,
        }
    }

    /// Returns this span, if it was [enabled] by the current [collector], or
    /// the [current span] (whose lexical distance may be further than expected),
    ///  if this span [is disabled].
    ///
    /// This method can be useful when propagating spans to spawned threads or
    /// [async tasks]. Consider the following:
    ///
    /// ```
    /// let _parent_span = tracing::info_span!("parent").entered();
    ///
    /// // ...
    ///
    /// let child_span = tracing::debug_span!("child");
    ///
    /// std::thread::spawn(move || {
    ///     let _entered = child_span.entered();
    ///
    ///     tracing::info!("spawned a thread!");
    ///
    ///     // ...
    /// });
    /// ```
    ///
    /// If the current [collector] enables the [`DEBUG`] level, then both
    /// the "parent" and "child" spans will be enabled. Thus, when the "spawned
    /// a thread!" event occurs, it will be inside of the "child" span. Because
    /// "parent" is the parent of "child", the event will _also_ be inside of
    /// "parent".
    ///
    /// However, if the collector only enables the [`INFO`] level, the "child"
    /// span will be disabled. When the thread is spawned, the
    /// `child_span.entered()` call will do nothing, since "child" is not
    /// enabled. In this case, the "spawned a thread!" event occurs outside of
    /// *any* span, since the "child" span was responsible for propagating its
    /// parent to the spawned thread.
    ///
    /// If this is not the desired behavior, `Span::or_current` can be used to
    /// ensure that the "parent" span is propagated in both cases, either as a
    /// parent of "child" _or_ directly. For example:
    ///
    /// ```
    /// let _parent_span = tracing::info_span!("parent").entered();
    ///
    /// // ...
    ///
    /// // If DEBUG is enabled, then "child" will be enabled, and `or_current`
    /// // returns "child". Otherwise, if DEBUG is not enabled, "child" will be
    /// // disabled, and `or_current` returns "parent".
    /// let child_span = tracing::debug_span!("child").or_current();
    ///
    /// std::thread::spawn(move || {
    ///     let _entered = child_span.entered();
    ///
    ///     tracing::info!("spawned a thread!");
    ///
    ///     // ...
    /// });
    /// ```
    ///
    /// When spawning [asynchronous tasks][async tasks], `Span::or_current` can
    /// be used similarly, in combination with [`instrument`]:
    ///
    /// ```
    /// use tracing::Instrument;
    /// # // lol
    /// # mod tokio {
    /// #     pub(super) fn spawn(_: impl std::future::Future) {}
    /// # }
    ///
    /// let _parent_span = tracing::info_span!("parent").entered();
    ///
    /// // ...
    ///
    /// let child_span = tracing::debug_span!("child");
    ///
    /// tokio::spawn(
    ///     async {
    ///         tracing::info!("spawned a task!");
    ///
    ///         // ...
    ///
    ///     }.instrument(child_span.or_current())
    /// );
    /// ```
    ///
    /// In general, `or_current` should be preferred over nesting an
    /// [`instrument`]  call inside of an [`in_current_span`] call, as using
    /// `or_current` will be more efficient.
    ///
    /// ```
    /// use tracing::Instrument;
    /// # // lol
    /// # mod tokio {
    /// #     pub(super) fn spawn(_: impl std::future::Future) {}
    /// # }
    /// async fn my_async_fn() {
    ///     // ...
    /// }
    ///
    /// let _parent_span = tracing::info_span!("parent").entered();
    ///
    /// // Do this:
    /// tokio::spawn(
    ///     my_async_fn().instrument(tracing::debug_span!("child").or_current())
    /// );
    ///
    /// // ...rather than this:
    /// tokio::spawn(
    ///     my_async_fn()
    ///         .instrument(tracing::debug_span!("child"))
    ///         .in_current_span()
    /// );
    /// ```
    ///
    /// [enabled]: crate::collect::Collect::enabled
    /// [collector]: crate::collect::Collect
    /// [current span]: Span::current
    /// [is disabled]: Span::is_disabled
    /// [`INFO`]: crate::Level::INFO
    /// [`DEBUG`]: crate::Level::DEBUG
    /// [async tasks]: std::task
    /// [`instrument`]: crate::instrument::Instrument::instrument
    /// [`in_current_span`]: crate::instrument::Instrument::in_current_span
    pub fn or_current(self) -> Self {
        if self.is_disabled() {
            return Self::current();
        }
        self
    }

    #[inline]
    fn do_enter(&self) {
        if let Some(inner) = self.inner.as_ref() {
            inner.collector.enter(&inner.id);
        }

        if_log_enabled! { crate::Level::TRACE, {
            if let Some(_meta) = self.meta {
                self.log(ACTIVITY_LOG_TARGET, log::Level::Trace, format_args!("-> {}", _meta.name()));
            }
        }}
    }

    // Called from [`Entered`] and [`EnteredSpan`] drops.
    //
    // Running this behaviour on drop rather than with an explicit function
    // call means that spans may still be exited when unwinding.
    #[inline]
    fn do_exit(&self) {
        if let Some(inner) = self.inner.as_ref() {
            inner.collector.exit(&inner.id);
        }

        if_log_enabled! { crate::Level::TRACE, {
            if let Some(_meta) = self.meta {
                self.log(ACTIVITY_LOG_TARGET, log::Level::Trace, format_args!("<- {}", _meta.name()));
            }
        }}
    }

    /// Executes the given function in the context of this span.
    ///
    /// If this span is enabled, then this function enters the span, invokes `f`
    /// and then exits the span. If the span is disabled, `f` will still be
    /// invoked, but in the context of the currently-executing span (if there is
    /// one).
    ///
    /// Returns the result of evaluating `f`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tracing::{trace, span, Level};
    /// let my_span = span!(Level::TRACE, "my_span");
    ///
    /// my_span.in_scope(|| {
    ///     // this event occurs within the span.
    ///     trace!("i'm in the span!");
    /// });
    ///
    /// // this event occurs outside the span.
    /// trace!("i'm not in the span!");
    /// ```
    ///
    /// Calling a function and returning the result:
    /// ```
    /// # use tracing::{info_span, Level};
    /// fn hello_world() -> String {
    ///     "Hello world!".to_owned()
    /// }
    ///
    /// let span = info_span!("hello_world");
    /// // the span will be entered for the duration of the call to
    /// // `hello_world`.
    /// let a_string = span.in_scope(hello_world);
    ///
    pub fn in_scope<F: FnOnce() -> T, T>(&self, f: F) -> T {
        let _enter = self.enter();
        f()
    }

    /// Returns a [`Field`](super::field::Field) for the field with the
    /// given `name`, if one exists,
    pub fn field<Q: ?Sized>(&self, field: &Q) -> Option<field::Field>
    where
        Q: field::AsField,
    {
        self.metadata().and_then(|meta| field.as_field(meta))
    }

    /// Returns true if this `Span` has a field for the given
    /// [`Field`](super::field::Field) or field name.
    #[inline]
    pub fn has_field<Q: ?Sized>(&self, field: &Q) -> bool
    where
        Q: field::AsField,
    {
        self.field(field).is_some()
    }

    /// Records that the field described by `field` has the value `value`.
    ///
    /// This may be used with [`field::Empty`] to declare fields whose values
    /// are not known when the span is created, and record them later:
    /// ```
    /// use tracing::{trace_span, field};
    ///
    /// // Create a span with two fields: `greeting`, with the value "hello world", and
    /// // `parting`, without a value.
    /// let span = trace_span!("my_span", greeting = "hello world", parting = field::Empty);
    ///
    /// // ...
    ///
    /// // Now, record a value for parting as well.
    /// // (note that the field name is passed as a string slice)
    /// span.record("parting", &"goodbye world!");
    /// ```
    /// However, it may also be used to record a _new_ value for a field whose
    /// value was already recorded:
    /// ```
    /// use tracing::info_span;
    /// # fn do_something() -> Result<(), ()> { Err(()) }
    ///
    /// // Initially, let's assume that our attempt to do something is going okay...
    /// let span = info_span!("doing_something", is_okay = true);
    /// let _e = span.enter();
    ///
    /// match do_something() {
    ///     Ok(something) => {
    ///         // ...
    ///     }
    ///     Err(_) => {
    ///         // Things are no longer okay!
    ///         span.record("is_okay", &false);
    ///     }
    /// }
    /// ```
    ///
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    ///
    /// **Note**: The fields associated with a span are part of its [`Metadata`].
    /// The [`Metadata`] describing a particular
    /// span is constructed statically when the span is created and cannot be extended later to
    /// add new fields. Therefore, you cannot record a value for a field that was not specified
    /// when the span was created:
    ///
    /// </pre></div>
    ///
    /// ```
    /// use tracing::{trace_span, field};
    ///
    /// // Create a span with two fields: `greeting`, with the value "hello world", and
    /// // `parting`, without a value.
    /// let span = trace_span!("my_span", greeting = "hello world", parting = field::Empty);
    ///
    /// // ...
    ///
    /// // Now, you try to record a value for a new field, `new_field`, which was not
    /// // declared as `Empty` or populated when you created `span`.
    /// // You won't get any error, but the assignment will have no effect!
    /// span.record("new_field", &"interesting_value_you_really_need");
    ///
    /// // Instead, all fields that may be recorded after span creation should be declared up front,
    /// // using field::Empty when a value is not known, as we did for `parting`.
    /// // This `record` call will indeed replace field::Empty with "you will be remembered".
    /// span.record("parting", &"you will be remembered");
    /// ```
    ///
    /// [`field::Empty`]: super::field::Empty
    /// [`Metadata`]: super::Metadata
    pub fn record<Q: ?Sized, V>(&self, field: &Q, value: &V) -> &Self
    where
        Q: field::AsField,
        V: field::Value,
    {
        if let Some(meta) = self.meta {
            if let Some(field) = field.as_field(meta) {
                self.record_all(
                    &meta
                        .fields()
                        .value_set(&[(&field, Some(value as &dyn field::Value))]),
                );
            }
        }

        self
    }

    /// Records all the fields in the provided `ValueSet`.
    pub fn record_all(&self, values: &field::ValueSet<'_>) -> &Self {
        let record = Record::new(values);
        if let Some(ref inner) = self.inner {
            inner.record(&record);
        }

        if let Some(_meta) = self.meta {
            if_log_enabled! { *_meta.level(), {
                let target = if record.is_empty() {
                    LIFECYCLE_LOG_TARGET
                } else {
                    _meta.target()
                };
                self.log(target, level_to_log!(*_meta.level()), format_args!("{}{}", _meta.name(), FmtValues(&record)));
            }}
        }

        self
    }

    /// Returns `true` if this span was disabled by the collector and does not
    /// exist.
    ///
    /// See also [`is_none`].
    ///
    /// [`is_none`]: Span::is_none()
    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.inner.is_none()
    }

    /// Returns `true` if this span was constructed by [`Span::none`] and is
    /// empty.
    ///
    /// If `is_none` returns `true` for a given span, then [`is_disabled`] will
    /// also return `true`. However, when a span is disabled by the collector
    /// rather than constructed by `Span::none`, this method will return
    /// `false`, while `is_disabled` will return `true`.
    ///
    /// [`is_disabled`]: Span::is_disabled()
    #[inline]
    pub fn is_none(&self) -> bool {
        self.is_disabled() && self.meta.is_none()
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
    ///
    /// # Examples
    ///
    /// Setting a `follows_from` relationship with a `Span`:
    /// ```
    /// # use tracing::{span, Id, Level, Span};
    /// let span1 = span!(Level::INFO, "span_1");
    /// let span2 = span!(Level::DEBUG, "span_2");
    /// span2.follows_from(&span1);
    /// ```
    ///
    /// Setting a `follows_from` relationship with the current span:
    /// ```
    /// # use tracing::{span, Id, Level, Span};
    /// let span = span!(Level::INFO, "hello!");
    /// span.follows_from(&Span::current());
    /// ```
    ///
    /// Setting a `follows_from` relationship with an `Id`:
    /// ```
    /// # use tracing::{span, Id, Level, Span};
    /// let span = span!(Level::INFO, "hello!");
    /// let id = span.id();
    /// span.follows_from(id);
    /// ```
    pub fn follows_from(&self, from: impl Into<Option<Id>>) -> &Self {
        if let Some(ref inner) = self.inner {
            if let Some(from) = from.into() {
                inner.follows_from(&from);
            }
        }
        self
    }

    /// Returns this span's `Id`, if it is enabled.
    pub fn id(&self) -> Option<Id> {
        self.inner.as_ref().map(Inner::id)
    }

    /// Returns this span's `Metadata`, if it is enabled.
    pub fn metadata(&self) -> Option<&'static Metadata<'static>> {
        self.meta
    }

    #[cfg(feature = "log")]
    #[inline]
    fn log(&self, target: &str, level: log::Level, message: fmt::Arguments<'_>) {
        if let Some(meta) = self.meta {
            if level_to_log!(*meta.level()) <= log::max_level() {
                let logger = log::logger();
                let log_meta = log::Metadata::builder().level(level).target(target).build();
                if logger.enabled(&log_meta) {
                    if let Some(ref inner) = self.inner {
                        logger.log(
                            &log::Record::builder()
                                .metadata(log_meta)
                                .module_path(meta.module_path())
                                .file(meta.file())
                                .line(meta.line())
                                .args(format_args!("{}; span={}", message, inner.id.into_u64()))
                                .build(),
                        );
                    } else {
                        logger.log(
                            &log::Record::builder()
                                .metadata(log_meta)
                                .module_path(meta.module_path())
                                .file(meta.file())
                                .line(meta.line())
                                .args(message)
                                .build(),
                        );
                    }
                }
            }
        }
    }

    /// Invokes a function with a reference to this span's ID and collector.
    ///
    /// if this span is enabled, the provided function is called, and the result is returned.
    /// If the span is disabled, the function is not called, and this method returns `None`
    /// instead.
    pub fn with_collector<T>(&self, f: impl FnOnce((&Id, &Dispatch)) -> T) -> Option<T> {
        self.inner
            .as_ref()
            .map(|inner| f((&inner.id, &inner.collector)))
    }
}

impl cmp::PartialEq for Span {
    fn eq(&self, other: &Self) -> bool {
        match (&self.meta, &other.meta) {
            (Some(this), Some(that)) => {
                this.callsite() == that.callsite() && self.inner == other.inner
            }
            _ => false,
        }
    }
}

impl Hash for Span {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.inner.hash(hasher);
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut span = f.debug_struct("Span");
        if let Some(meta) = self.meta {
            span.field("name", &meta.name())
                .field("level", &meta.level())
                .field("target", &meta.target());

            if let Some(ref inner) = self.inner {
                span.field("id", &inner.id());
            } else {
                span.field("disabled", &true);
            }

            if let Some(ref path) = meta.module_path() {
                span.field("module_path", &path);
            }

            if let Some(ref line) = meta.line() {
                span.field("line", &line);
            }

            if let Some(ref file) = meta.file() {
                span.field("file", &file);
            }
        } else {
            span.field("none", &true);
        }

        span.finish()
    }
}

impl<'a> From<&'a Span> for Option<&'a Id> {
    fn from(span: &'a Span) -> Self {
        span.inner.as_ref().map(|inner| &inner.id)
    }
}

impl<'a> From<&'a Span> for Option<Id> {
    fn from(span: &'a Span) -> Self {
        span.inner.as_ref().map(Inner::id)
    }
}

impl<'a> From<&'a EnteredSpan> for Option<&'a Id> {
    fn from(span: &'a EnteredSpan) -> Self {
        span.inner.as_ref().map(|inner| &inner.id)
    }
}

impl<'a> From<&'a EnteredSpan> for Option<Id> {
    fn from(span: &'a EnteredSpan) -> Self {
        span.inner.as_ref().map(Inner::id)
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if let Some(Inner {
            ref id,
            ref collector,
        }) = self.inner
        {
            collector.try_close(id.clone());
        }

        if let Some(_meta) = self.meta {
            if_log_enabled! { crate::Level::TRACE, {
                self.log(
                    LIFECYCLE_LOG_TARGET,
                    log::Level::Trace,
                    format_args!("-- {}", _meta.name()),
                );
            }}
        }
    }
}

// ===== impl Inner =====

impl Inner {
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
    fn follows_from(&self, from: &Id) {
        self.collector.record_follows_from(&self.id, from)
    }

    /// Returns the span's ID.
    fn id(&self) -> Id {
        self.id.clone()
    }

    fn record(&self, values: &Record<'_>) {
        self.collector.record(&self.id, values)
    }

    fn new(id: Id, collector: &Dispatch) -> Self {
        Inner {
            id,
            collector: collector.clone(),
        }
    }
}

impl cmp::PartialEq for Inner {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for Inner {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Clone for Inner {
    fn clone(&self) -> Self {
        Inner {
            id: self.collector.clone_span(&self.id),
            collector: self.collector.clone(),
        }
    }
}

// ===== impl Entered =====

impl EnteredSpan {
    /// Returns this span's `Id`, if it is enabled.
    pub fn id(&self) -> Option<Id> {
        self.inner.as_ref().map(Inner::id)
    }

    /// Exits this span, returning the underlying [`Span`].
    #[inline]
    pub fn exit(mut self) -> Span {
        // One does not simply move out of a struct with `Drop`.
        let span = mem::replace(&mut self.span, Span::none());
        span.do_exit();
        span
    }
}

impl Deref for EnteredSpan {
    type Target = Span;

    #[inline]
    fn deref(&self) -> &Span {
        &self.span
    }
}

impl<'a> Drop for Entered<'a> {
    #[inline]
    fn drop(&mut self) {
        self.span.do_exit()
    }
}

impl Drop for EnteredSpan {
    #[inline]
    fn drop(&mut self) {
        self.span.do_exit()
    }
}

/// Technically, `Entered` (or `EnteredSpan`) _can_ implement both `Send` *and*
/// `Sync` safely. It doesn't, because it has a `PhantomNotSend` field,
/// specifically added in order to make it `!Send`.
///
/// Sending an `Entered` guard between threads cannot cause memory unsafety.
/// However, it *would* result in incorrect behavior, so we add a
/// `PhantomNotSend` to prevent it from being sent between threads. This is
/// because it must be *dropped* on the same thread that it was created;
/// otherwise, the span will never be exited on the thread where it was entered,
/// and it will attempt to exit the span on a thread that may never have entered
/// it. However, we still want them to be `Sync` so that a struct holding an
/// `Entered` guard can be `Sync`.
///
/// Thus, this is totally safe.
#[derive(Debug)]
struct PhantomNotSend {
    ghost: PhantomData<*mut ()>,
}

#[allow(non_upper_case_globals)]
const PhantomNotSend: PhantomNotSend = PhantomNotSend { ghost: PhantomData };

/// # Safety
///
/// Trivially safe, as `PhantomNotSend` doesn't have any API.
unsafe impl Sync for PhantomNotSend {}

#[cfg(feature = "log")]
struct FmtValues<'a>(&'a Record<'a>);

#[cfg(feature = "log")]
impl<'a> fmt::Display for FmtValues<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = Ok(());
        let mut is_first = true;
        self.0.record(&mut |k: &field::Field, v: &dyn fmt::Debug| {
            res = write!(f, "{} {}={:?}", if is_first { ";" } else { "" }, k, v);
            is_first = false;
        });
        res
    }
}

#[cfg(feature = "log")]
struct FmtAttrs<'a>(&'a Attributes<'a>);

#[cfg(feature = "log")]
impl<'a> fmt::Display for FmtAttrs<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = Ok(());
        let mut is_first = true;
        self.0.record(&mut |k: &field::Field, v: &dyn fmt::Debug| {
            res = write!(f, "{} {}={:?}", if is_first { ";" } else { "" }, k, v);
            is_first = false;
        });
        res
    }
}

#[cfg(test)]
mod test {
    use super::*;

    trait AssertSend: Send {}
    impl AssertSend for Span {}

    trait AssertSync: Sync {}
    impl AssertSync for Span {}
    impl AssertSync for Entered<'_> {}
    impl AssertSync for EnteredSpan {}
}
