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

#[macro_use]
extern crate lazy_static;

use std::{borrow::Borrow, fmt, slice};

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

pub mod callsite;
pub mod dispatcher;
pub mod field;
pub mod span;
pub mod subscriber;

pub use self::{
    callsite::Callsite,
    dispatcher::Dispatch,
    field::{AsValue, IntoValue, Key, Value},
    span::{Attributes as SpanAttributes, Id as SpanId, Span},
    subscriber::Subscriber,
};
use field::BorrowedValue;

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
pub struct Event<'a> {
    /// The span ID of the span in which this event occurred.
    pub parent: Option<SpanId>,

    /// The IDs of a set of spans which are causally linked with this event, but
    /// are not its direct parent.
    pub follows_from: &'a [SpanId],

    /// Metadata describing this event.
    pub meta: &'a Meta<'a>,

    /// The values of the fields on this event.
    ///
    /// The names of these fields are defined in the event's metadata. Each
    /// index in this array corresponds to the name at the same index in
    /// `self.meta.field_names`.
    pub field_values: &'a [&'a dyn AsValue],

    /// A textual message describing the event that occurred.
    pub message: fmt::Arguments<'a>,
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
/// **Note**: The implementation of `PartialEq` for `Meta` is address-based,
/// *not* structural equality. This is intended as a performance optimization,
/// as metadata are intended to be created statically once per callsite.
/// [`Span`]: ::span::Span
/// [`Event`]: ::Event
/// [`Subscriber`]: ::Subscriber
#[derive(Clone, Debug, Eq, Hash)]
pub struct Meta<'a> {
    // TODO: The fields on this type are currently `pub` because it must be able
    // to be constructed statically by macros. However, when `const fn`s are
    // available on stable Rust, this will no longer be necessary. Thus, these
    // fields should be made private when `const fn` is stable.
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

    /// Whether this metadata escribes a [`Span`] or an [`Event`].
    ///
    /// [`Span`]: ::span::Span
    /// [`Event`]: ::Event
    pub kind: MetaKind,
}

/// Indicates whether a set of [metadata] describes a [`Span`] or an [`Event`].
///
/// [metadata]: ::Meta
/// [`Span`]: ::span::Span
/// [`Event`]: ::Event
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct MetaKind(KindInner);

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum KindInner {
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
            kind: MetaKind::SPAN,
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
            kind: MetaKind::EVENT,
        }
    }

    /// Returns true if this metadata corresponds to an event.
    pub fn is_event(&self) -> bool {
        self.kind.is_event()
    }

    /// Returns true if this metadata corresponds to a span.
    pub fn is_span(&self) -> bool {
        self.kind.is_span()
    }

    /// Returns an iterator over the fields defined by this set of metadata.
    pub fn fields(&'a self) -> impl Iterator<Item = field::Key<'a>> {
        (0..self.field_names.len()).map(move |i| Key::new(i, self))
    }

    /// Returns a [`Key`](::field::Key) to the field corresponding to `name`, if
    /// one exists, or `None` if no such field exists.
    pub fn key_for<Q>(&'a self, name: &Q) -> Option<field::Key<'a>>
    where
        Q: Borrow<str>,
    {
        let name = &name.borrow();
        self.field_names
            .iter()
            .position(|f| f == name)
            .map(|i| Key::new(i, self))
    }

    /// Returns `true` if `self` contains a field for the given `key`.
    pub fn contains_key(&self, key: &field::Key<'a>) -> bool {
        key.metadata() == self && key.as_usize() <= self.field_names.len()
    }
}

impl<'a> PartialEq for Meta<'a> {
    #[inline]
    fn eq(&self, other: &Meta<'a>) -> bool {
        ::std::ptr::eq(self, other)
    }
}

// ===== impl Event =====

impl<'a> Event<'a> {
    /// Notifies the currently active [`Subscriber`] of an `Event` at the given
    /// [`Callsite`].
    ///
    /// If the given callsite is enabled, a  new `Event` will be constructed
    /// with the provided `field_values` and `message`, following from the
    /// provided span IDs. The subscriber will then observe that event.
    ///
    /// If the callsite is not enabled, this function does nothing.
    ///
    /// [`Subscriber`]: ::subscriber::Subscriber
    /// [`Callsite`]: ::callsite::Callsite
    pub fn observe(
        callsite: &'a dyn callsite::Callsite,
        field_values: &[&dyn field::AsValue],
        follows_from: &[SpanId],
        message: fmt::Arguments<'a>,
    ) {
        let dispatch = Dispatch::current();
        if callsite.is_enabled(&dispatch) {
            dispatch.observe_event(&Event {
                parent: SpanId::current(),
                follows_from,
                meta: callsite.metadata(),
                field_values,
                message,
            });
        }
    }

    /// Returns an iterator over the names of all the fields on this `Event`.
    pub fn field_names(&self) -> slice::Iter<&'a str> {
        self.meta.field_names.iter()
    }

    /// Returns true if this event has a field for the specified `key`.
    #[inline]
    pub fn has_field(&self, key: &field::Key) -> bool {
        self.meta.contains_key(key)
    }

    /// Borrows the value of the field named `name`, if it exists. Otherwise,
    /// returns `None`.
    pub fn field(&self, key: &field::Key) -> Option<field::BorrowedValue> {
        if !self.has_field(key) {
            return None;
        }
        self.field_values
            .get(key.as_usize())
            .map(|&val| field::borrowed(val))
    }

    /// Returns an iterator over all the field names and values on this event.
    pub fn fields<'b: 'a>(&'b self) -> impl Iterator<Item = (field::Key<'a>, BorrowedValue<'b>)> {
        self.meta.fields().filter_map(move |key| {
            let val = self.field(&key)?;
            Some((key, val))
        })
    }

    /// Returns a struct that can be used to format all the fields on this
    /// `Event` with `fmt::Debug`.
    pub fn debug_fields<'b: 'a>(&'b self) -> DebugFields<'b, 'b, Self, BorrowedValue<'a>> {
        DebugFields(self)
    }
}

impl<'a> IntoIterator for &'a Event<'a> {
    type Item = (field::Key<'a>, BorrowedValue<'a>);
    type IntoIter = Box<Iterator<Item = (field::Key<'a>, BorrowedValue<'a>)> + 'a>; // TODO: unbox
    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.fields())
    }
}

/// Formats the key-value fields of a `Span` or `Event` with `fmt::Debug`.
pub struct DebugFields<'a, 'b: 'a, I: 'a, T: 'a>(&'a I)
where
    &'a I: IntoIterator<Item = (field::Key<'b>, T)>;

impl<'a, 'b: 'a, I: 'a, T: 'a> fmt::Debug for DebugFields<'a, 'b, I, T>
where
    &'a I: IntoIterator<Item = (field::Key<'b>, T)>,
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0
            .into_iter()
            .fold(&mut f.debug_struct(""), |s, (key, value)| {
                let name = key.name().unwrap_or("???");
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

// ===== impl MetaKind =====

impl MetaKind {
    /// Returns `true` if this metadata corresponds to a `Span`.
    pub fn is_span(&self) -> bool {
        match self {
            MetaKind(KindInner::Span) => true,
            _ => false,
        }
    }

    /// Returns `true` if this metadata corresponds to an `Event`.
    pub fn is_event(&self) -> bool {
        match self {
            MetaKind(KindInner::Event) => true,
            _ => false,
        }
    }

    /// The `MetaKind` for `Span` metadata.
    pub const SPAN: Self = MetaKind(KindInner::Span);

    /// The `MetaKind` for `Event` metadata.
    pub const EVENT: Self = MetaKind(KindInner::Event);
}
