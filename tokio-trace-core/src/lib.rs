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
//! begins executing in that context, and to _exit_ the span when switching to
//! another context. The span in which a thread is currently executing is
//! referred to as the _current_ span.
//!
//! Spans form a tree structure --- unless it is a root span, all spans have a
//! _parent_, and may have one or more _children_. When a new span is created,
//! the current span becomes the new span's parent. The total execution time of
//! a span consists of the time spent in that span and in the entire subtree
//! represented by its children. Thus, a parent span always lasts for at least
//! as long as the longest-executing span in its subtree.
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
//! [`Span`]: span/struct.Span
//! [`Event`]: struct.Event.html
//! [`Subscriber`]: subscriber/trait.Subscriber.html
//! [`observe_event`]: subscriber/trait.Subscriber.html#tymethod.observe_event
//! [`enter`]: subscriber/trait.Subscriber.html#tymethod.enter
//! [`exit`]: subscriber/trait.Subscriber.html#tymethod.exit
//! [`enabled`]: subscriber/trait.Subscriber.html#tymethod.enabled
//! [metadata]: struct.Meta.html
#![warn(missing_docs)]

use std::{fmt, slice};

#[macro_export]
macro_rules! callsite {
    (span: $name:expr, $( $field_name:ident ),*) => ({
        callsite!(@ $crate::Meta {
            name: Some($name),
            target: module_path!(),
            level: $crate::Level::Trace,
            module_path: Some(module_path!()),
            file: Some(file!()),
            line: Some(line!()),
            field_names: &[ $(stringify!($field_name)),* ],
            kind: $crate::Kind::Span,
        })
    });
    (event: $lvl:expr, $( $field_name:ident ),*) =>
        (callsite!(event: $lvl, target: module_path!(), $( $field_name ),* ));
    (event: $lvl:expr, target: $target:expr, $( $field_name:ident ),*) => ({
        callsite!(@ $crate::Meta {
            name: None,
            target: $target,
            level: $lvl,
            module_path: Some(module_path!()),
            file: Some(file!()),
            line: Some(line!()),
            field_names: &[ $(stringify!($field_name)),* ],
            kind: $crate::Kind::Event,
        })
    });
    (@ $meta:expr ) => ({
        use $crate::{callsite, Meta};
        static META: Meta<'static> = $meta;
        thread_local! {
            static CACHE: callsite::Cache<'static> = callsite::Cache::new(&META);
        }
        callsite::Callsite::new(&CACHE)
    })
}

/// Describes the level of verbosity of a `Span` or `Event`.
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

#[doc(hidden)]
pub mod callsite;

pub mod dispatcher;
pub mod span;
pub mod subscriber;
pub mod value;

pub use self::{
    dispatcher::Dispatch,
    span::{Data as SpanData, Id as SpanId, Span},
    subscriber::Subscriber,
    value::{AsValue, IntoValue, Value},
};
use value::BorrowedValue;

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
/// **Note**: `Event` must be generic over two lifetimes, that of `Event` itself
/// (the `'event` lifetime) *and* the lifetime of the event's metadata (the
/// `'meta` lifetime), which must be at least as long as the event's lifetime.
/// This is because the metadata may live as long as the lifetime, or it may be
/// `'static` and reused for all `Event`s generated from a particular source
/// code location (as is the case when the event is produced by the `event!`
/// macro). Consumers of `Event` probably do not need to actually care about
/// these lifetimes, however.
///
/// [`Span`]: ::span::Span
pub struct Event<'event, 'meta> {
    /// The span ID of the span in which this event occurred.
    pub parent: Option<SpanId>,

    /// The IDs of a set of spans which are causally linked with this event, but
    /// are not its direct parent.
    pub follows_from: &'event [SpanId],

    /// Metadata describing this event.
    pub meta: &'meta Meta<'meta>,

    /// The values of the fields on this event.
    ///
    /// The names of these fields are defined in the event's metadata. Each
    /// index in this array corresponds to the name at the same index in
    /// `self.meta.field_names`.
    pub field_values: &'event [&'event dyn AsValue],

    /// A textual message describing the event that occurred.
    pub message: fmt::Arguments<'event>,
}

/// Metadata describing a [`Span`] or [`Event`].
///
/// This includes the source code location where the span or event occurred, the
/// names of its fields, et cetera.
///
/// Metadata is used by [`Subscriber`]s when filtering spans and events, and it
/// may also be used as part of their data payload.
///
/// When created by the `event!` or `span!` macro, the metadata describing a
/// particular event or span is constructed statically and exists as a single
/// static instance. Thus, the overhead of  creating the metadata is
/// _significantly_ lower than that of creating the actual span or event.
/// Therefore, filtering is based on metadata, rather than  on the constructed
/// span or event.
///
/// [`Span`]: ::span::Span
/// [`Event`]: ::Event
/// [`Subscriber`]: ::Subscriber
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Meta<'a> {
    /// If this metadata describes a span, the name of the span.
    pub name: Option<&'a str>,

    /// The part of the system that the span or event that this metadata
    /// describes occurred in.
    ///
    /// Typically, this is the module path, but alternate targets may be set
    /// when spans or events are constructed.
    pub target: &'a str,

    /// The level of verbosity of the described span or event.
    pub level: Level,

    /// The name of the Rust module where the span or event occurred, or `None`
    /// if this could not be determined.
    pub module_path: Option<&'a str>,

    /// The name of the source code file where the span or event occurred, or
    /// `None` if this could not be determined.
    pub file: Option<&'a str>,

    /// The line number in the source code file where the span or event
    /// occurred, or `None` if this could not be determined.
    pub line: Option<u32>,

    /// The names of the key-value fields attached to the described span or
    /// event.
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
    pub fn field<Q>(&self, name: Q) -> Option<value::BorrowedValue>
    where
        &'event str: PartialEq<Q>,
    {
        self.field_names()
            .position(|&field_name| field_name == name)
            .and_then(|i| self.field_values.get(i).map(|&val| value::borrowed(val)))
    }

    /// Returns an iterator over all the field names and values on this event.
    pub fn fields<'a: 'event>(&'a self) -> impl Iterator<Item = (&'event str, BorrowedValue<'a>)> {
        self.field_names()
            .enumerate()
            .filter_map(move |(idx, &name)| {
                self.field_values
                    .get(idx)
                    .map(|&val| (name, value::borrowed(val)))
            })
    }

    /// Returns a struct that can be used to format all the fields on this
    /// `Event` with `fmt::Debug`.
    pub fn debug_fields<'a: 'event>(&'a self) -> DebugFields<'a, Self, BorrowedValue<'event>> {
        DebugFields(self)
    }
}

impl<'a, 'm: 'a> IntoIterator for &'a Event<'a, 'm> {
    type Item = (&'a str, BorrowedValue<'a>);
    type IntoIter = Box<Iterator<Item = (&'a str, BorrowedValue<'a>)> + 'a>; // TODO: unbox
    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.fields())
    }
}

/// Formats the key-value fields of a `Span` or `Event` with `fmt::Debug`.
pub struct DebugFields<'a, I: 'a, T: 'a>(&'a I)
where
    &'a I: IntoIterator<Item = (&'a str, T)>;

impl<'a, I: 'a, T: 'a> fmt::Debug for DebugFields<'a, I, T>
where
    &'a I: IntoIterator<Item = (&'a str, T)>,
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0
            .into_iter()
            .fold(&mut f.debug_struct(""), |s, (name, value)| {
                s.field(name, &value)
            }).finish()
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
