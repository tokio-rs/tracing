//!
//! # Core Concepts
//!
//! The core of `tokio-trace`'s API is composed of `Event`s, `Span`s, and
//! `Subscriber`s. We'll cover these in turn.
//!
//! # `Span`s
//!
//! A [`Span`] represents a _period of time_ during which a program was executing
//! in some context. A thread of execution is said to _enter_ a span when it
//! begins executing in that context and _exit_s the span when switching to
//! another context. The span in which a thread is currently executing is
//! referred to as the _current_ span.
//!
//! Spans form a tree structure --- unless it is the root span, all spans have a
//! _parent_, and may have one or more _children_. When a new span is created,
//! the current span becomes the new span's parent. The total execution time of
//! a span consists of the time spent in that span and in the entire subtree
//! represented by its children. Thus, a parent span always lasts for at least
//! as long as the longest-executing span in its subtree.
//!
//! Furthermore, execution may enter and exit a span multiple times before that
//! span is _completed_. Consider, for example, a future which has an associated
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
//!         self.span.clone().enter(|| {
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
//! that span.
//!
//! In addition, data may be associated with spans. A span may have _fields_ ---
//! a set of key-value pairs describing the state of the program during that
//! span; an optional name, and metadata describing the source code location
//! where the span was originally entered.
//!
//! # Events
//!
//! An [`Event`] represents a _point_ in time. It signifies something that
//! happened while the trace was executing. `Event`s are comparable to the log
//! records emitted by unstructured logging code, but unlike a typical log line,
//! an `Event` always occurs within the context of a `Span`. Like a `Span`, it
//! may have fields, and implicitly inherits any of the fields present on its
//! parent span. Additionally, it may be linked with one or more additional
//! spans that are not its parent; in this case, the event is said to _follow
//! from_ those spans.
//!
//! Essentially, `Event`s exist to bridge the gap between traditional
//! unstructured logging and span-based tracing. Similar to log records, they
//! may be recorded at a number of levels, and can have unstructured,
//! human-readable messages; however, they also carry key-value data and exist
//! within the context of the tree of spans that comprise a trase. Thus,
//! individual log record-like events can be pinpointed not only in time, but
//! in the logical execution flow of the system.
//!
//! # `Subscriber`s
//!
//! As `Span`s and `Event`s occur, they are recorded or aggregated by
//! implementations of the [`Subscriber`] trait. `Subscriber`s are notified
//! when an `Event` takes place and when a `Span` is entered or exited. These
//! notifications are represented by the following `Subscriber` trait methods:
//! + [`observe_event`], called when an `Event` takes place,
//! + [`enter`], called when execution enters a `Span`,
//! + [`exit`], called when execution exits a `Span`
//!
//! In addition, subscribers may implement the [`enabled`] function to _filter_
//! the notifications they receive based on [metadata] describing each `Span`
//! or `Event`. If a call to `Subscriber::enabled` returns `false` for a given
//! set of metadata, that `Subscriber` will *not* be notified about the
//! corresponding `Span` or `Event`. For performance reasons, if no currently
//! active subscribers express  interest in a given set of metadata by returning
//! `true`, then the corresponding `Span` or `Event` will never be constructed.
//!
//! `Event`s and `Span`s are broadcast to `Subscriber`s by the [`Dispatcher`], a
//! special `Subscriber` implementation which broadcasts the notifications it
//! receives to a list of attached `Subscriber`s. The [`Dispatcher::builder`]
//! function returns a builder that can be used to attach `Subscriber`s to a
//! `Dispatcher` and initialize it.
//!
//! [`Span`]: span/struct.Span
//! [`Event`]: struct.Event.html
//! [`Subscriber`]: subscriber/trait.Subscriber.html
//! [`observe_event`]: subscriber/trait.Subscriber.html#tymethod.observe_event
//! [`enter`]: subscriber/trait.Subscriber.html#tymethod.enter
//! [`exit`]: subscriber/trait.Subscriber.html#tymethod.exit
//! [`enabled`]: subscriber/trait.Subscriber.html#tymethod.enabled
//! [metadata]: struct.Meta.html
//! [`Dispatcher`]: struct.Dispatcher.html
//! [`Dispatcher::builder`]: struct.Dispatcher.html#method.builder

extern crate futures;

use std::{fmt, slice};

#[doc(hidden)]
#[macro_export]
macro_rules! meta {
    (span: $name:expr, $( $field_name:ident ),*) => ({
        $crate::Meta {
            name: Some($name),
            target: module_path!(),
            level: $crate::Level::Trace,
            module_path: Some(module_path!()),
            file: Some(file!()),
            line: Some(line!()),
            field_names: &[ $(stringify!($field_name)),* ],
            kind: $crate::Kind::Span,
        }
    });
    (event: $lvl:expr, $( $field_name:ident ),*) =>
        (meta!(event: $lvl, target: module_path!(), $( $field_name ),* ));
    (event: $lvl:expr, target: $target:expr, $( $field_name:ident ),*) => ({
        $crate::Meta {
            name: None,
            target: $target,
            level: $lvl,
            module_path: Some(module_path!()),
            file: Some(file!()),
            line: Some(line!()),
            field_names: &[ $(stringify!($field_name)),* ],
            kind: $crate::Kind::Event,
        }
    });
}

// Cache the result of testing if a span or event with the given metadata is
// enabled by the current subscriber, so the filter doesn't have to be
// reapplied if we have already called `enabled`.
#[doc(hidden)]
#[macro_export]
macro_rules! cached_filter {
    ($meta:expr, $dispatcher:expr) => {
        {
            use std::sync::atomic::{ATOMIC_USIZE_INIT, AtomicUsize, Ordering};
            static FILTERED: AtomicUsize = ATOMIC_USIZE_INIT;
            const ENABLED: usize = 1;
            const DISABLED: usize = 2;
            if $dispatcher.should_invalidate_filter($meta) {
                let enabled = $dispatcher.enabled(&META);
                if enabled {
                    FILTERED.store(ENABLED, Ordering::Relaxed);
                } else {
                    FILTERED.store(DISABLED, Ordering::Relaxed);
                }
                enabled
            } else {
                match FILTERED.load(Ordering::Relaxed) {
                    // If there's a cached result, use that.
                    ENABLED => true,
                    DISABLED => false,
                    // Otherwise, this span has not yet been filtered, so call
                    // `enabled` now and store the result.
                    _ => {
                        let enabled = $dispatcher.enabled(&META);
                        if enabled {
                            FILTERED.store(ENABLED, Ordering::Relaxed);
                        } else {
                            FILTERED.store(DISABLED, Ordering::Relaxed);
                        }
                        enabled
                    },
                }
            }
        }
    }
}

/// Constructs a new span.
///
/// # Examples
///
/// Creating a new span with no fields:
/// ```
/// # #[macro_use]
/// # extern crate tokio_trace;
/// # fn main() {
/// let span = span!("my span");
/// span.enter(|| {
///     // do work inside the span...
/// });
/// # }
/// ```
///
/// Creating a span with fields:
/// ```
/// # #[macro_use]
/// # extern crate tokio_trace;
/// # fn main() {
/// span!("my span", foo = 2, bar = "a string").enter(|| {
///     // do work inside the span...
/// });
/// # }
/// ```
#[macro_export]
macro_rules! span {
    ($name:expr) => { span!($name,) };
    ($name:expr, $($k:ident = $val:expr),*) => {
        {
            use $crate::{Span, Subscriber, Dispatch, Meta};
            static META: Meta<'static> = meta! { span: $name, $( $k ),* };
            let dispatcher = Dispatch::current();
            if cached_filter!(&META, dispatcher) {
                Span::new(
                    dispatcher,
                    &META,
                    vec![ $(Box::new($val)),* ], // todo: wish this wasn't double-boxed...
                )
            } else {
                Span::new_disabled()
            }
        }
    }
}

#[macro_export]
macro_rules! event {
    (target: $target:expr, $lvl:expr, { $($k:ident = $val:expr),* }, $($arg:tt)+ ) => ({
        {
            use $crate::{SpanId, Subscriber, Dispatch, Meta, SpanData, Event, Value};
            static META: Meta<'static> = meta! { event:
                $lvl,
                target:
                $target, $( $k ),*
            };
            let dispatcher = Dispatch::current();
            if cached_filter!(&META, dispatcher) {
                let field_values: &[& dyn Value] = &[ $( & $val),* ];
                dispatcher.observe_event(&Event {
                    parent: SpanId::current(),
                    follows_from: &[],
                    meta: &META,
                    field_values: &field_values[..],
                    message: format_args!( $($arg)+ ),
                });
            }
        }
    });
    ($lvl:expr, { $($k:ident = $val:expr),* }, $($arg:tt)+ ) => (
        event!(target: module_path!(), $lvl, { $($k = $val),* }, $($arg)+)
    )
}

#[repr(usize)]
#[derive(Copy, Eq, Debug, Hash)]
pub enum Level {
    /// The "error" level.
    ///
    /// Designates very serious errors.
    Error = 1, // This way these line up with the discriminants for LevelFilter below
    /// The "warn" level.
    ///
    /// Designates hazardous situations.
    Warn,
    /// The "info" level.
    ///
    /// Designates useful information.
    Info,
    /// The "debug" level.
    ///
    /// Designates lower priority information.
    Debug,
    /// The "trace" level.
    ///
    /// Designates very low priority, often extremely verbose, information.
    Trace,
}

mod dispatcher;
pub mod span;
pub mod subscriber;

pub use self::{
    dispatcher::Dispatch,
    span::{Data as SpanData, Span, Id as SpanId},
    subscriber::Subscriber,
};

// XXX: im using fmt::Debug for prototyping purposes, it should probably leave.
pub trait Value: fmt::Debug + Send + Sync {
    // ... ?
}

impl<T> Value for T where T: fmt::Debug + Send + Sync {}

/// **Note**: `Event` must be generic over two lifetimes, that of `Event` itself
/// (the `'event` lifetime) *and* the lifetime of the event's metadata (the
/// `'meta` lifetime), which must be at least as long as the event's lifetime.
/// This is because the metadata may live as long as the lifetime, or it may be
/// `'static` and reused for all `Event`s generated from a particular source
/// code location (as is the case when the event is produced by the `event!`
/// macro). Consumers of `Event` probably do not need to actually care about
/// these lifetimes, however.
pub struct Event<'event, 'meta> {
    pub parent: Option<SpanId>,
    pub follows_from: &'event [SpanId],

    pub meta: &'meta Meta<'meta>,
    // TODO: agh box
    pub field_values: &'event [&'event dyn Value],
    pub message: fmt::Arguments<'event>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Meta<'a> {
    pub name: Option<&'a str>,
    pub target: &'a str,
    pub level: Level,

    pub module_path: Option<&'a str>,
    pub file: Option<&'a str>,
    pub line: Option<u32>,

    pub field_names: &'a [&'a str],

    #[doc(hidden)]
    pub kind: Kind,
}

#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Kind {
    Span,
    Event,
}

type StaticMeta = Meta<'static>;

// ===== impl Meta =====

impl<'a> Meta<'a> {

    /// Construct new metadata for a span, with a name, target, level, field
    /// names, and optional source code location.
    pub fn new_span(
        name: Option<&'a str>,
        target: &'a str,
        level: Level,
        module_path: Option<&'a str>,
        file: Option<&'a str>,
        line: Option<u32>,
        field_names: &'a [&'a str],
    ) -> Self {
        Self {
            name,
            target,
            level,
            module_path,
            file,
            line,
            field_names,
            kind: Kind::Span,
        }
    }

    /// Construct new metadata for an event, with a target, level, field names,
    /// and optional source code location.
    pub fn new_event(
        target: &'a str,
        level: Level,
        module_path: Option<&'a str>,
        file: Option<&'a str>,
        line: Option<u32>,
        field_names: &'a [&'a str],
    ) -> Self {
        Self {
            name: None,
            target,
            level,
            module_path,
            file,
            line,
            field_names,
            kind: Kind::Event,
        }
    }

    /// Returns true if this metadata corresponds to an event.
    pub fn is_event(&self) -> bool {
        match self.kind {
            Kind::Event => true,
            _ => false,
        }
    }

    /// Returns true if this metadata corresponds to a span.
    pub fn is_span(&self) -> bool {
        match self.kind {
            Kind::Span => true,
            _ => false,
        }
    }
}


// ===== impl Event =====

impl<'event, 'meta: 'event> Event<'event, 'meta> {
    /// Returns an iterator over the names of all the fields on this `Event`.
    pub fn field_names(&self) -> slice::Iter<&'event str> {
        self.meta.field_names.iter()
    }

    /// Borrows the value of the field named `name`, if it exists. Otherwise,
    /// returns `None`.
    pub fn field<Q>(&'event self, name: Q) -> Option<&'event dyn Value>
    where
        &'event str: PartialEq<Q>,
    {
        self.field_names()
            .position(|&field_name| field_name == name)
            .and_then(|i| self.field_values.get(i).map(|&val| val))
    }

    /// Returns an iterator over all the field names and values on this event.
    pub fn fields(
        &'event self,
    ) -> impl Iterator<Item = (&'event str, &'event dyn Value)> {
        self.field_names()
            .enumerate()
            .filter_map(move |(idx, &name)| {
                self.field_values.get(idx).map(|&val| (name, val))
            })
    }

    /// Returns a struct that can be used to format all the fields on this
    /// `Event` with `fmt::Debug`.
    pub fn debug_fields<'a: 'meta>(&'a self) -> DebugFields<'a, Self> {
        DebugFields(self)
    }
}

impl<'a, 'm: 'a> IntoIterator for &'a Event<'a, 'm> {
    type Item = (&'a str, &'a dyn Value);
    type IntoIter = Box<Iterator<Item = (&'a str, &'a dyn Value)> + 'a>; // TODO: unbox
    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.fields())
    }
}

pub struct DebugFields<'a, I: 'a>(&'a I)
where
    &'a I: IntoIterator<Item = (&'a str, &'a dyn Value)>;

impl<'a, I: 'a> fmt::Debug for DebugFields<'a, I>
where
    &'a I: IntoIterator<Item = (&'a str, &'a dyn Value)>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.into_iter()
            .fold(&mut f.debug_struct(""), |s, (name, value)| {
                s.field(name, &value)
            })
            .finish()
    }
}

// ===== impl Level =====

impl Clone for Level {
    #[inline]
    fn clone(&self) -> Level {
        *self
    }
}

impl PartialEq for Level {
    #[inline]
    fn eq(&self, other: &Level) -> bool {
        *self as usize == *other as usize
    }
}
