//! Subscribers collect and record trace data.
use {span, Event, IntoValue, Meta, SpanId};

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
/// - Observing spans as they are entered and exited, and events as they occur.
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
    fn new_span(&self, span: span::Data) -> span::Id;

    // // XXX: should this be a subscriber method or should it have its own type???
    // fn span_data(&self, id: &span::Id) -> Option<&span::Data>;

    /// Adds a new field to an existing span observed by this `Subscriber`.
    ///
    /// This is expected to return an error under the following conditions:
    /// - The span ID does not correspond to a span which currently exists.
    /// - The span does not have a field with the given name.
    /// - The span has a field with the given name, but the value has already
    ///   been set.
    fn add_value(
        &self,
        span: &span::Id,
        name: &'static str,
        value: &dyn IntoValue,
    ) -> Result<(), AddValueError>;

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
    /// follow from _b_), it should return a [`PriorError`].
    ///
    /// [`PriorError`]: PriorError
    fn add_prior_span(&self, span: &span::Id, follows: span::Id) -> Result<(), PriorError>;

    // === Filtering methods ==================================================

    /// Determines if a span or event with the specified [metadata] would be
    /// recorded.
    ///
    /// This is used by the dispatcher to avoid allocating for span construction
    /// if the span would be discarded anyway.
    ///
    /// [metadata]: ::Meta
    fn enabled(&self, metadata: &Meta) -> bool;

    /// Returns `true` if the cached result to a call to [`enabled`] for a span
    /// with the given metadata is still valid.
    ///
    /// By default, this function assumes that cached filter results will remain
    /// valid, but should be overridden when this is not the case.
    ///
    /// If this returns `false`, then the prior value may be used. `Subscriber`s
    /// which require their filters to be run every time an event occurs or a
    /// span is entered/exited should always return `true`.
    ///
    /// For example, suppose a sampling subscriber is implemented by
    /// incrementing a counter every time `enabled` is called and only returning
    /// `true` when the counter is divisible by a specified sampling rate. If
    /// that subscriber returns `false` from `should_invalidate_filter`, then
    /// the filter will not be re-evaluated once it has been applied to a given
    /// set of metadata. Thus, the counter will not be incremented, and the span
    /// or event that correspands to the metadata will never be `enabled`.
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
    ///
    /// [metadata]: ::Meta [`enabled`]: ::Subscriber::enabled
    fn should_invalidate_filter(&self, _metadata: &Meta) -> bool {
        false
    }

    // === Notification methods ===============================================

    /// Records that an [`Event`] has occurred.
    ///
    /// When an `Event` takes place, this function is called to notify the
    /// subscriber of that event.
    ///
    /// Note that this function is generic over a pair of lifetimes because the
    /// `Event` type is. See the documentation for [`Event`] for details.
    ///
    /// [`Event`]: ../struct.Event.html
    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>);

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
    fn enter(&self, span: SpanId, state: span::State);

    /// Records that a [`Span`] has been exited.
    ///
    /// When exiting a span, this method is called to notify the subscriber
    /// that the span has been exited. The subscriber is provided with the
    /// [`SpanId`] that identifies the exited span, and the current [`State`]
    /// of the span.
    ///
    /// The state may be used to determine whether the span may be entered again
    /// (`State::Idle`), or if the span has completed and will not be entered
    /// again (`State::Done`).
    ///
    /// [`Span`]: ::span::Span
    /// [`SpanId`]: ::span::Id
    /// [`State`]: ::span::State
    fn exit(&self, span: SpanId, state: span::State);
}

/// Errors which may prevent a value from being successfully added to a span.
// TODO: before releasing core 0.1 this needs to be made private, to avoid
// future breaking changes.
#[derive(Clone, Debug)]
pub enum AddValueError {
    /// The span with the given ID does not exist.
    NoSpan,
    /// The span exists, but does not have the specified field.
    NoField,
    /// The named field already has a value.
    FieldAlreadyExists,
}

/// Errors which may prevent a prior span from being added to a span.
// TODO: before releasing core 0.1 this needs to be made private, to avoid
// future breaking changes.
#[derive(Clone, Debug)]
pub enum PriorError {
    /// The span with the given ID does not exist.
    /// TODO: can this error type be generalized between `PriorError` and
    /// `AddValueError`?
    NoSpan(SpanId),
    /// The span that this span follows from does not exist (it has no ID).
    NoPreceedingId,
}

#[cfg(any(test, feature = "test-support"))]
pub use self::test_support::*;

#[cfg(any(test, feature = "test-support"))]
mod test_support {
    #![allow(missing_docs)]

    use super::*;
    use span::{self, MockSpan};
    use {Event, IntoValue, Meta, SpanData, SpanId};

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
    }

    struct Running<F: Fn(&Meta) -> bool> {
        spans: Mutex<HashMap<SpanId, SpanData>>,
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
            self.expected
                .push_back(Expect::Enter(span.with_state(::span::State::Running)));
            self
        }

        pub fn exit(mut self, span: MockSpan) -> Self {
            self.expected.push_back(Expect::Exit(span));
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

    impl<F: Fn(&Meta) -> bool> Subscriber for Running<F> {
        fn enabled(&self, meta: &Meta) -> bool {
            (self.filter)(meta)
        }

        fn add_value(
            &self,
            _span: &span::Id,
            _name: &'static str,
            _value: &dyn IntoValue,
        ) -> Result<(), AddValueError> {
            // TODO: it should be possible to expect values...
            Ok(())
        }

        fn add_prior_span(&self, _span: &span::Id, _follows: span::Id) -> Result<(), PriorError> {
            // TODO: it should be possible to expect spans to follow from other spans
            Ok(())
        }

        fn new_span(&self, span: SpanData) -> span::Id {
            let id = self.ids.fetch_add(1, Ordering::SeqCst);
            let id = span::Id::from_u64(id as u64);
            self.spans.lock().unwrap().insert(id.clone(), span);
            id
        }

        fn observe_event<'event, 'meta: 'event>(&self, _event: &'event Event<'event, 'meta>) {
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
            }
        }

        fn enter(&self, span: span::Id, state: span::State) {
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
                    if let Some(expected_state) = expected_span.state {
                        assert_eq!(expected_state, state);
                    }
                    // TODO: expect fields
                }
                Some(Expect::Exit(expected_span)) => panic!(
                    "expected to exit span {:?}, but entered span {:?} instead",
                    expected_span.name,
                    span.name()
                ),
            }
        }

        fn exit(&self, span: span::Id, state: span::State) {
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
                    if let Some(expected_state) = expected_span.state {
                        assert_eq!(expected_state, state);
                    }
                    // TODO: expect fields
                }
            }
        }
    }
}
