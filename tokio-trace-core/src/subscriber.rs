//! Subscribers collect and record trace data.
use {field, span, Event, Meta, SpanId};

use std::{error::Error, fmt};

/// Trait representing the functions required to collect trace data.
///
/// Crates that provide implementations of methods for collecting or recording
/// trace data should implement the `Subscriber` interface. This trait is
/// intended to represent fundamental primitives for collecting trace events and
/// spans --- other libraries may offer utility functions and types to make
/// subscriber implementations more modular or improve the ergonomics of writing
/// subscribers.
///
/// A subscriber is responsible for the following:
/// - Registering new spans as they are created, and providing them with span
///   IDs. Implicitly, this means the subscriber may determine the strategy for
///   determining span equality.
/// - Recording the attachment of field values and follows-from annotations to
///   spans.
/// - Filtering spans and events, and determining when those filters must be
///   invalidated.
/// - Observing spans as they are entered, exited, and closed, and events as
///   they occur.
///
/// When a span is entered or exited, the subscriber is provided only with the
/// [`SpanId`] with which it tagged that span when it was created. This means
/// that it is up to the subscriber to determine whether or not span _data_ ---
/// the fields and metadata describing the span --- should be stored. The
/// [`new_span`] function is called when a new span is created, and at that
/// point, the subscriber _may_ choose to store the associated data if it will
/// be referenced again. However, if the data has already been recorded and will
/// not be needed by the implementations of `enter` and `exit`, the subscriber
/// may freely discard that data without allocating space to store it.
///
/// [`SpanId`]: ::span::Id
/// [`new_span`]: ::Span::new_span
pub trait Subscriber {
    // === Span registry methods ==============================================

    /// Registers a new callsite with this subscriber, returning whether or not
    /// the subscriber is interested in being notified about the callsite.
    ///
    /// By default, this function assumes that the subscriber's filter
    /// represents an unchanging view of its interest in the callsite. However,
    /// if this is not the case, subscribers may override this function to
    /// indicate different interests, or to implement behaviour that should run
    /// once for every callsite.
    ///
    /// This function is guaranteed to be called exactly once per callsite on
    /// every active subscriber. The subscriber may store the keys to fields it
    /// cares in order to reduce the cost of accessing fields by name,
    /// preallocate storage for that callsite, or perform any other actions it
    /// wishes to perform once for each callsite.
    ///
    /// The subscriber should then return an [`Interest`](Interest), indicating
    /// whether it is interested in being notified about that callsite in the
    /// future. This may be `Always` indicating that the subscriber always
    /// wishes to be notified about the callsite, and its filter need not be
    /// re-evaluated; `Sometimes`, indicating that the subscriber may sometimes
    /// care about the callsite but not always (such as when sampling), or
    /// `Never`, indicating that the subscriber never wishes to be notified about
    /// that callsite. If all active subscribers return `Never`, a callsite will
    /// never be enabled unless a new subscriber expresses interest in it.
    ///
    /// `Subscriber`s which require their filters to be run every time an event
    /// occurs or a span is entered/exited should return `Interest::Sometimes`.
    ///
    /// For example, suppose a sampling subscriber is implemented by
    /// incrementing a counter every time `enabled` is called and only returning
    /// `true` when the counter is divisible by a specified sampling rate. If
    /// that subscriber returns `Interest::Always` from `register_callsite`, then
    /// the filter will not be re-evaluated once it has been applied to a given
    /// set of metadata. Thus, the counter will not be incremented, and the span
    /// or event that correspands to the metadata will never be `enabled`.
    ///
    /// Similarly, if a `Subscriber` has a filtering strategy that can be
    /// changed dynamically at runtime, it would need to re-evaluate that filter
    /// if the cached results have changed.
    // TODO: there should be a function to request all callsites be
    // re-registered?
    ///
    /// A subscriber which manages fanout to multiple other subscribers
    /// should proxy this decision to all of its child subscribers,
    /// returning `Interest::Never` only if _all_ such children return
    /// `Interest::Never`. If the set of subscribers to which spans are
    /// broadcast may change dynamically, the subscriber should also never
    /// return `Interest::Never`, as a new subscriber may be added that _is_
    /// interested.
    ///
    /// **Note**: If a subscriber returns `Interest::Never` for a particular
    /// callsite, it _may_ still see spans and events originating from that
    /// callsite, if another subscriber expressed interest in it.
    /// [metadata]: ::Meta [`enabled`]: ::Subscriber::enabled
    fn register_callsite(&self, metadata: &Meta) -> Interest {
        Interest::from_filter(self.enabled(metadata))
    }

    /// Record the construction of a new [`Span`], returning a a new [span ID] for
    /// the span being constructed.
    ///
    /// Span IDs are used to uniquely identify spans within the context
    /// of a subscriber, so span equality will be based on the returned
    /// ID. Thus, if the subscriber wishes for all spans with the same
    /// metadata to be considered equal, it should return the same ID
    /// every time it is given a particular set of metadata. Similarly,
    /// if it wishes for two separate instances of a span with the same
    /// metadata to *not* be equal, it should return a distinct ID every
    /// time this function is called, regardless of the metadata.
    ///
    /// Subscribers which do not rely on the implementations of `PartialEq`,
    /// `Eq`, and `Hash` for `Span`s are free to return span IDs with value 0
    /// from all calls to this function, if they so choose.
    ///
    /// [span ID]: ::span::Id
    /// [`Span`]: ::span::Span
    fn new_span(&self, span: span::Attributes) -> span::Id;

    /// Adds a new field to an existing span observed by this `Subscriber`.
    ///
    /// This is expected to return an error under the following conditions:
    /// - The span ID does not correspond to a span which currently exists.
    /// - The span does not have a field with the given name.
    /// - The span has a field with the given name, but the value has already
    ///   been set.
    fn record(
        &self,
        span: &span::Id,
        key: &field::Key,
        value: &dyn field::Value,
    ) -> Result<(), RecordError>;

    /// Adds an indication that `span` follows from the span with the id
    /// `follows`.
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
    /// If the subscriber has spans corresponding to the given IDs, it should
    /// record this relationship in whatever way it deems necessary. Otherwise,
    /// if one or both of the given span IDs do not correspond to spans that the
    /// subscriber knows about, or if a cyclical relationship would be created
    /// (i.e., some span _a_ which proceeds some other span _b_ may not also
    /// follow from _b_), it should return a [`FollowsError`].
    ///
    /// [`FollowsError`]: FollowsError
    fn add_follows_from(&self, span: &span::Id, follows: span::Id) -> Result<(), FollowsError>;

    // === Filtering methods ==================================================

    /// Determines if a span or event with the specified [metadata] would be
    /// recorded.
    ///
    /// This is used by the dispatcher to avoid allocating for span construction
    /// if the span would be discarded anyway.
    ///
    /// [metadata]: ::Meta
    fn enabled(&self, metadata: &Meta) -> bool;

    // === Notification methods ===============================================

    /// Records that an [`Event`] has occurred.
    ///
    /// When an `Event` takes place, this function is called to notify the
    /// subscriber of that event.
    ///
    /// [`Event`]: ../struct.Event.html
    fn observe_event<'a>(&self, event: &'a Event<'a>);

    /// Records that a [`Span`] has been entered.
    ///
    /// When entering a span, this method is called to notify the subscriber
    /// that the span has been entered. The subscriber is provided with the
    /// [`SpanId`] that identifies the entered span, and the current [`State`]
    /// of the span.
    ///
    /// [`Span`]: ::span::Span
    /// [`SpanId`]: ::span::Id
    /// [`State`]: ::span::State
    fn enter(&self, span: span::Id);

    /// Records that a [`Span`] has been exited.
    ///
    /// When exiting a span, this method is called to notify the subscriber
    /// that the span has been exited. The subscriber is provided with the
    /// [`SpanId`] that identifies the exited span.
    ///
    /// Exiting a span does not imply that the span will not be re-entered.
    /// [`Span`]: ::span::Span
    /// [`SpanId`]: ::span::Id
    fn exit(&self, span: span::Id);

    /// Records that a [`Span`] has been closed.
    ///
    /// When exiting a span, this method is called to notify the subscriber
    /// that the span has been closed. The subscriber is provided with the
    /// [`SpanId`] that identifies the closed span.
    ///
    /// Unlike `exit`, this method implies that the span will not be entered
    /// again. The subscriber is free to use that guarantee as it sees fit (such
    /// as garbage-collecting any cached data related to that span, if
    /// necessary).
    ///
    /// [`Span`]: ::span::Span
    /// [`SpanId`]: ::span::Id
    fn close(&self, span: span::Id);
}

/// Indicates a `Subscriber`'s interest in a particular callsite.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Interest(InterestKind);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum InterestKind {
    Never = 0,
    Sometimes = 1,
    Always = 2,
}

/// Errors which may prevent a value from being successfully added to a span.
#[derive(Debug)]
pub struct RecordError {
    kind: RecordErrorKind,
}

#[derive(Debug)]
enum RecordErrorKind {
    /// The span with the given ID does not exist.
    NoSpan(SpanId),
    /// The span exists, but does not have the specified field.
    NoField,
    /// The named field already has a value.
    AlreadyExists,
    /// An error occurred recording the field.
    Record,
}

/// Errors which may prevent a span from following another span.
#[derive(Debug)]
pub struct FollowsError {
    kind: FollowsErrorKind,
}

#[derive(Debug)]
enum FollowsErrorKind {
    /// The span with the given ID does not exist.
    /// TODO: can this error type be generalized between `FollowsError` and
    /// `RecordError`?
    NoSpan(SpanId),
    /// The span that this span follows from does not exist (it has no ID).
    NoPreceedingId,
}

impl Interest {
    /// Construct a new `Interest` from the result of evaluating a filter.
    pub fn from_filter(filter: bool) -> Self {
        match filter {
            true => Interest::ALWAYS,
            false => Interest::NEVER,
        }
    }

    /// Indicates that the subscriber is never interested in being notified
    /// about a callsite.
    ///
    /// If all active subscribers are `NEVER` interested in a callsite, it will
    /// be completely disabled unless a new subscriber becomes active.
    pub const NEVER: Interest = Interest(InterestKind::Never);

    /// Indicates that the subscriber is sometimes interested in being
    /// notified about a callsite.
    ///
    /// If all active subscribers are `sometimes` or `never` interested in a
    /// callsite, the currently active subscriber will be asked to filter that
    /// callsite every time it creates a span or event. This will be the case
    /// until a subscriber expresses that it is `always` interested in the
    /// callsite.
    pub const SOMETIMES: Interest = Interest(InterestKind::Sometimes);

    /// Indicates that the subscriber is always interested in being
    /// notified about a callsite.
    ///
    /// If any subscriber expresses that it is `ALWAYS` interested in a given
    /// callsite, then the callsite will always be enabled.
    pub const ALWAYS: Interest = Interest(InterestKind::Always);

    /// Constructs a new `Interest` from a `usize`.
    #[inline]
    pub fn from_usize(u: usize) -> Option<Self> {
        match u {
            0 => Some(Interest::NEVER),
            1 => Some(Interest::SOMETIMES),
            2 => Some(Interest::ALWAYS),
            _ => None,
        }
    }

    /// Returns an `usize` representing this `Interest`.
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

// ===== impl RecordError =====

impl RecordError {
    /// Returns an error indicating that no span exists for the given `id`.
    pub fn no_span(id: SpanId) -> Self {
        Self {
            kind: RecordErrorKind::NoSpan(id),
        }
    }

    /// Returns an error indicating that the span exists, but does not have the
    /// specified field.
    pub fn no_field() -> Self {
        Self {
            kind: RecordErrorKind::NoField,
        }
    }

    /// Return an error indicating that the named field already has a value.
    pub fn already_exists() -> Self {
        Self {
            kind: RecordErrorKind::AlreadyExists,
        }
    }

    /// Returns an error indicating that an error occurred recording the field.
    pub fn record() -> Self {
        Self {
            kind: RecordErrorKind::Record,
        }
    }

    /// Returns `true` if this error was due to no span existing for the
    /// specified field.
    pub fn is_no_span(&self) -> bool {
        match self.kind {
            RecordErrorKind::NoSpan(_) => true,
            _ => false,
        }
    }

    /// Returns `true` if this error was due to no field existing with the
    /// specified name.
    pub fn is_no_field(&self) -> bool {
        match self.kind {
            RecordErrorKind::NoField => true,
            _ => false,
        }
    }

    /// Returns `true` if this error was due to the named field already
    /// existing.
    pub fn is_already_exists(&self) -> bool {
        match self.kind {
            RecordErrorKind::AlreadyExists => true,
            _ => false,
        }
    }

    /// Returns `true` if this error was due to an error occurring while
    /// recording the field.
    pub fn is_record(&self) -> bool {
        match self.kind {
            RecordErrorKind::Record => true,
            _ => false,
        }
    }
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            RecordErrorKind::NoSpan(ref id) => write!(f, "no span exists with id {:?}", id),
            RecordErrorKind::NoField => f.pad("no such field exists"),
            RecordErrorKind::AlreadyExists => f.pad("field already has a value"),
            RecordErrorKind::Record => f.pad("an error occurred recording the field"),
        }
    }
}

impl Error for RecordError {}

impl From<fmt::Error> for RecordError {
    fn from(_e: fmt::Error) -> Self {
        RecordError::record()
    }
}

// ===== impl FollowsError =====

impl FollowsError {
    /// Returns an error indicating that no span exists for the given `id`.
    pub fn no_following_span(id: SpanId) -> Self {
        Self {
            kind: FollowsErrorKind::NoSpan(id),
        }
    }

    /// Returns an error indicating the preceeding span does not exist.
    pub fn no_preceeding_span() -> Self {
        Self {
            kind: FollowsErrorKind::NoPreceedingId,
        }
    }

    /// Returns `true` if this error was due to the following span not existing.
    pub fn is_no_following_span(&self) -> bool {
        match self.kind {
            FollowsErrorKind::NoSpan(_) => true,
            _ => false,
        }
    }

    /// Returns `true` if this error was due to the preceeding span not existing.
    pub fn is_no_preceeding_span(&self) -> bool {
        match self.kind {
            FollowsErrorKind::NoPreceedingId => true,
            _ => false,
        }
    }
}

impl fmt::Display for FollowsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            FollowsErrorKind::NoSpan(ref id) => write!(f, "no span exists with id {:?}", id),
            FollowsErrorKind::NoPreceedingId => f.pad("the preceeding span does not exist"),
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use self::test_support::*;

#[cfg(any(test, feature = "test-support"))]
mod test_support {
    #![allow(missing_docs)]

    use super::*;
    use span::{self, MockSpan};
    use {field, Event, Meta, SpanAttributes, SpanId};

    use std::{
        collections::{HashMap, VecDeque},
        sync::{
            atomic::{AtomicUsize, Ordering},
            Mutex,
        },
    };

    struct ExpectEvent {
        // TODO: implement
    }

    enum Expect {
        #[allow(dead_code)] // TODO: implement!
        Event(ExpectEvent),
        Enter(MockSpan),
        Exit(MockSpan),
        Close(MockSpan),
        Nothing,
    }

    struct Running<F: Fn(&Meta) -> bool> {
        spans: Mutex<HashMap<SpanId, SpanAttributes>>,
        expected: Mutex<VecDeque<Expect>>,
        ids: AtomicUsize,
        filter: F,
    }

    pub struct MockSubscriber<F: Fn(&Meta) -> bool> {
        expected: VecDeque<Expect>,
        filter: F,
    }

    pub fn mock() -> MockSubscriber<fn(&Meta) -> bool> {
        MockSubscriber {
            expected: VecDeque::new(),
            filter: (|_: &Meta| true) as for<'r, 's> fn(&'r Meta<'s>) -> _,
        }
    }

    impl<F: Fn(&Meta) -> bool> MockSubscriber<F> {
        pub fn enter(mut self, span: MockSpan) -> Self {
            self.expected.push_back(Expect::Enter(span));
            self
        }

        pub fn exit(mut self, span: MockSpan) -> Self {
            self.expected.push_back(Expect::Exit(span));
            self
        }

        pub fn close(mut self, span: MockSpan) -> Self {
            self.expected.push_back(Expect::Close(span));
            self
        }

        pub fn done(mut self) -> Self {
            self.expected.push_back(Expect::Nothing);
            self
        }

        pub fn with_filter<G>(self, filter: G) -> MockSubscriber<G>
        where
            G: Fn(&Meta) -> bool,
        {
            MockSubscriber {
                filter,
                expected: self.expected,
            }
        }

        pub fn run(self) -> impl Subscriber {
            Running {
                spans: Mutex::new(HashMap::new()),
                expected: Mutex::new(self.expected),
                ids: AtomicUsize::new(0),
                filter: self.filter,
            }
        }
    }

    impl<'a, F> field::Record for &'a Running<F>
    where
        F: Fn(&Meta) -> bool,
        F: 'a,
    {
        fn record_fmt(
            &mut self,
            _field: &field::Key,
            _value: ::std::fmt::Arguments,
        ) -> Result<(), ::subscriber::RecordError> {
            // TODO: it would be nice to be able to expect field values...
            Ok(())
        }
    }

    impl<F: Fn(&Meta) -> bool> Subscriber for Running<F> {
        fn enabled(&self, meta: &Meta) -> bool {
            (self.filter)(meta)
        }

        fn record(
            &self,
            _span: &span::Id,
            _key: &field::Key,
            _value: &dyn field::Value,
        ) -> Result<(), ::subscriber::RecordError> {
            Ok(())
        }

        fn add_follows_from(
            &self,
            _span: &span::Id,
            _follows: span::Id,
        ) -> Result<(), FollowsError> {
            // TODO: it should be possible to expect spans to follow from other spans
            Ok(())
        }

        fn new_span(&self, span: SpanAttributes) -> span::Id {
            let id = self.ids.fetch_add(1, Ordering::SeqCst);
            let id = span::Id::from_u64(id as u64);
            self.spans.lock().unwrap().insert(id.clone(), span);
            id
        }

        fn observe_event<'a>(&self, _event: &'a Event<'a>) {
            match self.expected.lock().unwrap().pop_front() {
                None => {}
                Some(Expect::Event(_)) => unimplemented!(),
                Some(Expect::Enter(expected_span)) => panic!(
                    "expected to enter span {:?}, but got an event",
                    expected_span.name
                ),
                Some(Expect::Exit(expected_span)) => panic!(
                    "expected to exit span {:?} but got an event",
                    expected_span.name
                ),
                Some(Expect::Close(expected_span)) => panic!(
                    "expected to close span {:?} but got an event",
                    expected_span.name,
                ),
                Some(Expect::Nothing) => {
                    panic!("expected nothing else to happen, but got an event")
                }
            }
        }

        fn enter(&self, span: span::Id) {
            println!("enter: {:?}", span);
            let spans = self.spans.lock().unwrap();
            let span = spans
                .get(&span)
                .unwrap_or_else(|| panic!("no span for ID {:?}", span));
            match self.expected.lock().unwrap().pop_front() {
                None => {}
                Some(Expect::Event(_)) => panic!(
                    "expected an event, but entered span {:?} instead",
                    span.name()
                ),
                Some(Expect::Enter(expected_span)) => {
                    if let Some(name) = expected_span.name {
                        assert_eq!(name, span.name());
                    }
                    // TODO: expect fields
                }
                Some(Expect::Exit(expected_span)) => panic!(
                    "expected to exit span {:?}, but entered span {:?} instead",
                    expected_span.name,
                    span.name()
                ),
                Some(Expect::Close(expected_span)) => panic!(
                    "expected to close span {:?}, but entered span {:?} instead",
                    expected_span.name,
                    span.name()
                ),
                Some(Expect::Nothing) => panic!(
                    "expected nothing else to happen, but entered span {:?}",
                    span.name(),
                ),
            }
        }

        fn exit(&self, span: span::Id) {
            println!("exit: {:?}", span);
            let spans = self.spans.lock().unwrap();
            let span = spans
                .get(&span)
                .unwrap_or_else(|| panic!("no span for ID {:?}", span));
            match self.expected.lock().unwrap().pop_front() {
                None => {}
                Some(Expect::Event(_)) => panic!(
                    "expected an event, but exited span {:?} instead",
                    span.name()
                ),
                Some(Expect::Enter(expected_span)) => panic!(
                    "expected to enter span {:?}, but exited span {:?} instead",
                    expected_span.name,
                    span.name()
                ),
                Some(Expect::Exit(expected_span)) => {
                    if let Some(name) = expected_span.name {
                        assert_eq!(name, span.name());
                    }
                    // TODO: expect fields
                }
                Some(Expect::Close(expected_span)) => panic!(
                    "expected to close span {:?}, but exited span {:?} instead",
                    expected_span.name,
                    span.name()
                ),
                Some(Expect::Nothing) => panic!(
                    "expected nothing else to happen, but exited span {:?}",
                    span.name(),
                ),
            }
        }

        fn close(&self, span: span::Id) {
            let spans = self.spans.lock().unwrap();
            let span = spans
                .get(&span)
                .unwrap_or_else(|| panic!("no span for ID {:?}", span));
            match self.expected.lock().unwrap().pop_front() {
                None => {}
                Some(Expect::Event(_)) => panic!(
                    "expected an event, but closed span {:?} instead",
                    span.name()
                ),
                Some(Expect::Enter(expected_span)) => panic!(
                    "expected to enter span {:?}, but closed span {:?} instead",
                    expected_span.name,
                    span.name()
                ),
                Some(Expect::Exit(expected_span)) => panic!(
                    "expected to exit span {:?}, but closed span {:?} instead",
                    expected_span.name,
                    span.name()
                ),
                Some(Expect::Close(expected_span)) => {
                    if let Some(name) = expected_span.name {
                        assert_eq!(name, span.name());
                    }
                    // TODO: expect fields
                }
                Some(Expect::Nothing) => panic!(
                    "expected nothing else to happen, but closed span {:?}",
                    span.name(),
                ),
            }
        }
    }
}
