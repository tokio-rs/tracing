//! Subscribers collect and record trace data.
use {field, Meta, Span};

use std::fmt;

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
/// [`Id`] with which it tagged that span when it was created. This means
/// that it is up to the subscriber to determine whether or not span _data_ ---
/// the fields and metadata describing the span --- should be stored. The
/// [`new_span`] function is called when a new span is created, and at that
/// point, the subscriber _may_ choose to store the associated data if it will
/// be referenced again. However, if the data has already been recorded and will
/// not be needed by the implementations of `enter` and `exit`, the subscriber
/// may freely discard that data without allocating space to store it.
///
/// [`Id`]: ::Id
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
        match self.enabled(metadata) {
            true => Interest::ALWAYS,
            false => Interest::NEVER,
        }
    }

    /// Record the construction of a new [`Span`], returning a new ID
    /// for the span being constructed.
    ///
    /// Unlike [`new_span`], this function is always called with span
    /// [metadata] which are valid for the `'static` lifetime.
    ///
    /// This function defaults to simply calling `self.new_span()`, but if the
    /// subscriber wishes to do something with the the known-`'static` span
    /// metadata (such as storing a reference to them in some collection) it
    /// may override the default implementation to do so. It may then generate a
    /// new ID for that span, either by calling `new_span`, or through a different
    /// method from the ID generation for events.
    ///
    /// [`Span`]: ::span::Span
    /// [`new_span`]: ::subscriber::Subscriber::new_span
    /// [metadata]: ::metadata::Meta
    fn new_static(&self, metadata: &'static Meta<'static>) -> Span {
        self.new_span(metadata)
    }

    /// Record the construction of a new [`Span`], returning a new ID for the
    /// span being constructed.
    ///
    /// IDs are used to uniquely identify spans and events within the context of a
    /// subscriber, so span equality will be based on the returned ID. Thus, if
    /// the subscriber wishes for all spans with the same metadata to be
    /// considered equal, it should return the same ID every time it is given a
    /// particular set of metadata. Similarly, if it wishes for two separate
    /// instances of a span with the same metadata to *not* be equal, it should
    /// return a distinct ID every time this function is called, regardless of
    /// the metadata.
    ///
    /// Subscribers which do not rely on the implementations of `PartialEq`,
    /// `Eq`, and `Hash` for `Span`s are free to return span IDs with value 0
    /// from all calls to this function, if they so choose.
    ///
    /// [`Span`]: ::span::Span
    fn new_span(&self, metadata: &Meta) -> Span;

    /// Record a signed 64-bit integer value.
    ///
    /// This defaults to calling `self.record_fmt()`; implementations wishing to
    /// provide behaviour specific to signed integers may override the default
    /// implementation.
    ///
    /// If recording the field is invalid (i.e. the span ID doesn't exist, the
    /// field has already been recorded, and so on), the subscriber may silently
    /// do nothing.
    fn record_i64(&self, span: &Span, field: &field::Key, value: i64) {
        self.record_debug(span, field, &value)
    }

    /// Record an umsigned 64-bit integer value.
    ///
    /// This defaults to calling `self.record_fmt()`; implementations wishing to
    /// provide behaviour specific to unsigned integers may override the default
    /// implementation.
    ///
    /// If recording the field is invalid (i.e. the span ID doesn't exist, the
    /// field has already been recorded, and so on), the subscriber may silently
    /// do nothing.
    fn record_u64(&self, span: &Span, field: &field::Key, value: u64) {
        self.record_debug(span, field, &value)
    }

    /// Record a boolean value.
    ///
    /// This defaults to calling `self.record_fmt()`; implementations wishing to
    /// provide behaviour specific to booleans may override the default
    /// implementation.
    ///
    /// If recording the field is invalid (i.e. the span ID doesn't exist, the
    /// field has already been recorded, and so on), the subscriber may silently
    /// do nothing.
    fn record_bool(&self, span: &Span, field: &field::Key, value: bool) {
        self.record_debug(span, field, &value)
    }

    /// Record a string value.
    ///
    /// This defaults to calling `self.record_str()`; implementations wishing to
    /// provide behaviour specific to strings may override the default
    /// implementation.
    ///
    /// If recording the field is invalid (i.e. the span ID doesn't exist, the
    /// field has already been recorded, and so on), the subscriber may silently
    /// do nothing.
    fn record_str(&self, span: &Span, field: &field::Key, value: &str) {
        self.record_debug(span, field, &value)
    }

    /// Record a value implementing `fmt::Debug`.
    ///
    /// If recording the field is invalid (i.e. the span ID doesn't exist, the
    /// field has already been recorded, and so on), the subscriber may silently
    /// do nothing.
    fn record_debug(&self, span: &Span, field: &field::Key, value: &fmt::Debug);

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
    /// follow from _b_), it may silently do nothing.
    fn add_follows_from(&self, span: &Span, follows: Span);

    // === Filtering methods ==================================================

    /// Determines if a span with the specified [metadata] would be
    /// recorded.
    ///
    /// This is used by the dispatcher to avoid allocating for span construction
    /// if the span would be discarded anyway.
    ///
    /// [metadata]: ::Meta
    fn enabled(&self, metadata: &Meta) -> bool;

    // === Notification methods ===============================================

    /// Records that a [`Span`] has been entered.
    ///
    /// When entering a span, this method is called to notify the subscriber
    /// that the span has been entered. The subscriber is provided with a handle
    /// to the entered span, and should return its handle to the span that was
    /// currently executing prior to entering the new span.
    ///
    /// [`Span`]: ::span::Span
    /// [`Id`]: ::Id
    /// [`State`]: ::span::State
    fn enter(&self, span: &Span);

    /// Records that a [`Span`] has been exited.
    ///
    /// When exiting a span, this method is called to notify the subscriber
    /// that the span has been exited. The subscriber is provided with the
    /// [`Id`] that identifies the exited span, and a handle to the
    /// previously executing span, which should become the new current span. The
    /// subscriber should return its handle to the exited span.
    ///
    /// Exiting a span does not imply that the span will not be re-entered.
    /// [`Span`]: ::span::Span
    /// [`Id`]: ::Id
    fn exit(&self, span: &Span);

    /// Notifies the subscriber that a [`Span`] handle with the given [`Id`] has
    /// been cloned.
    ///
    /// This function is guaranteed to only be called with span IDs that were
    /// returned by this subscriber's `new_span` function.
    ///
    /// Note that the default implementation of this function this is just the
    /// identity function, passing through the identifier. However, it can be
    /// used in conjunction with [`drop_span`] to track the number of handles
    /// capable of `enter`ing a span. When all the handles have been dropped
    /// (i.e., `drop_span` has been called one more time than `clone_span` for a
    /// given ID), the subscriber may assume that the span will not be entered
    /// again. It is then free to deallocate storage for data associated with
    /// that span, write data from that span to IO, and so on.
    ///
    /// For more unsafe situations, however, if `id` is itself a pointer of some
    /// kind this can be used as a hook to "clone" the pointer, depending on
    /// what that means for the specified pointer.
    ///
    /// [`Id`]: ::span::Id,
    /// [`drop_span`]: ::subscriber::Subscriber::drop_span
    fn clone_span(&self, id: &Span) -> Span {
        id.clone()
    }

    /// Notifies the subscriber that a [`Span`] handle with the given [`Id`] has
    /// been dropped.
    ///
    /// This function is guaranteed to only be called with span IDs that were
    /// returned by this subscriber's `new_span` function.
    ///
    /// It's guaranteed that if this function has been called once more than the
    /// number of times `clone_span` was called with the same `id`, then no more
    /// `Span`s using that `id` exist. This means that it can be used in
    /// conjunction with [`clone_span`] to track the number of handles
    /// capable of `enter`ing a span. When all the handles have been dropped
    /// (i.e., `drop_span` has been called one more time than `clone_span` for a
    /// given ID), the subscriber may assume that the span will not be entered
    /// again. It is then free to deallocate storage for data associated with
    /// that span, write data from that span to IO, and so on.
    ///
    /// **Note**: since this function is called when spans are dropped,
    /// implementations should ensure that they are unwind-safe. Panicking from
    /// inside of a `drop_span` function may cause a double panic, if the span
    /// was dropped due to a thread unwinding.
    ///
    /// [`Id`]: ::span::Id,
    /// [`drop_span`]: ::subscriber::Subscriber::drop_span
    fn drop_span(&self, id: Span) {
        let _ = id;
    }
}

/// Indicates a `Subscriber`'s interest in a particular callsite.
#[derive(Clone, Debug)]
pub struct Interest(InterestKind);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum InterestKind {
    Never = 0,
    Sometimes = 1,
    Always = 2,
}

impl Interest {
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
    /// callsite every time it creates a span. This will be the case
    /// until a subscriber expresses that it is `always` interested in the
    /// callsite.
    pub const SOMETIMES: Interest = Interest(InterestKind::Sometimes);

    /// Indicates that the subscriber is always interested in being
    /// notified about a callsite.
    ///
    /// If any subscriber expresses that it is `ALWAYS` interested in a given
    /// callsite, then the callsite will always be enabled.
    pub const ALWAYS: Interest = Interest(InterestKind::Always);

    /// Returns `true` if the subscriber is never interested in being notified
    /// about this callsite.
    pub fn is_never(&self) -> bool {
        match self.0 {
            InterestKind::Never => true,
            _ => false,
        }
    }

    /// Returns `true` if the subscriber is sometimes interested in being notified
    /// about this callsite.
    pub fn is_sometimes(&self) -> bool {
        match self.0 {
            InterestKind::Sometimes => true,
            _ => false,
        }
    }

    /// Returns `true` if the subscriber is always interested in being notified
    /// about this callsite.
    pub fn is_always(&self) -> bool {
        match self.0 {
            InterestKind::Always => true,
            _ => false,
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use self::test_support::*;

#[cfg(any(test, feature = "test-support"))]
mod test_support {
    #![allow(missing_docs)]

    use super::*;
    use span::MockSpan;
    use {field, Meta, Span};

    use std::{
        collections::{HashMap, VecDeque},
        fmt,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        },
    };

    #[derive(Debug, Eq, PartialEq)]
    struct ExpectEvent {
        // TODO: implement
    }

    #[derive(Debug, Eq, PartialEq)]
    enum Expect {
        #[allow(dead_code)] // TODO: implement!
        Event(ExpectEvent),
        Enter(MockSpan),
        Exit(MockSpan),
        CloneSpan(MockSpan),
        DropSpan(MockSpan),
        Nothing,
    }

    enum SpanOrEvent {
        Span {
            span: &'static Meta<'static>,
            refs: usize,
        },
        Event,
    }

    struct Running<F: Fn(&Meta) -> bool> {
        spans: Mutex<HashMap<Span, SpanOrEvent>>,
        expected: Arc<Mutex<VecDeque<Expect>>>,
        ids: AtomicUsize,
        filter: F,
    }

    pub struct MockSubscriber<F: Fn(&Meta) -> bool> {
        expected: VecDeque<Expect>,
        filter: F,
    }

    pub struct MockHandle(Arc<Mutex<VecDeque<Expect>>>);

    pub fn mock() -> MockSubscriber<fn(&Meta) -> bool> {
        MockSubscriber {
            expected: VecDeque::new(),
            filter: (|_: &Meta| true) as for<'r, 's> fn(&'r Meta<'s>) -> _,
        }
    }

    impl SpanOrEvent {
        fn name(&self) -> &str {
            match self {
                SpanOrEvent::Span { span, .. } => span.name,
                SpanOrEvent::Event => "event",
            }
        }
    }

    impl<F: Fn(&Meta) -> bool> MockSubscriber<F> {
        pub fn enter(mut self, span: MockSpan) -> Self {
            self.expected.push_back(Expect::Enter(span));
            self
        }

        pub fn event(mut self) -> Self {
            // TODO: expect message/fields!
            self.expected.push_back(Expect::Event(ExpectEvent {}));
            self
        }

        pub fn exit(mut self, span: MockSpan) -> Self {
            self.expected.push_back(Expect::Exit(span));
            self
        }

        pub fn clone_span(mut self, span: MockSpan) -> Self {
            self.expected.push_back(Expect::CloneSpan(span));
            self
        }

        pub fn drop_span(mut self, span: MockSpan) -> Self {
            self.expected.push_back(Expect::DropSpan(span));
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
            let (subscriber, _) = self.run_with_handle();
            subscriber
        }

        pub fn run_with_handle(self) -> (impl Subscriber, MockHandle) {
            let expected = Arc::new(Mutex::new(self.expected));
            let handle = MockHandle(expected.clone());
            let subscriber = Running {
                spans: Mutex::new(HashMap::new()),
                expected,
                ids: AtomicUsize::new(0),
                filter: self.filter,
            };
            (subscriber, handle)
        }
    }

    impl<F: Fn(&Meta) -> bool> Subscriber for Running<F> {
        fn enabled(&self, meta: &Meta) -> bool {
            (self.filter)(meta)
        }

        fn record_debug(&self, span: &Span, field: &field::Key, value: &fmt::Debug) {
            // TODO: it would be nice to be able to expect field values...
        }

        fn add_follows_from(&self, _span: &Span, _follows: Span) {
            // TODO: it should be possible to expect spans to follow from other spans
        }

        fn new_span(&self, _attrs: &Meta) -> Span {
            let id = self.ids.fetch_add(1, Ordering::SeqCst);
            let id = Span::from_u64(id as u64);
            println!("new_span: id={:?};", id);
            self.spans
                .lock()
                .unwrap()
                .insert(id.clone(), SpanOrEvent::Event);
            id
        }

        fn new_static(&self, span: &'static Meta<'static>) -> Span {
            let id = self.ids.fetch_add(1, Ordering::SeqCst);
            let id = Span::from_u64(id as u64);
            println!("new_static: {}; id={:?};", span.name, id);
            self.spans
                .lock()
                .unwrap()
                .insert(id.clone(), SpanOrEvent::Span { span, refs: 1 });
            id
        }

        fn enter(&self, id: &Span) {
            let spans = self.spans.lock().unwrap();
            if let Some(span_or_event) = spans.get(id) {
                println!("enter: {}; id={:?};", span_or_event.name(), id);
                match (span_or_event, self.expected.lock().unwrap().pop_front()) {
                    (_, None) => {}
                    (SpanOrEvent::Event, _) => panic!("events should never be entered!"),
                    (SpanOrEvent::Span { span, .. }, Some(Expect::Event(_))) => panic!(
                        "expected an event, but entered span {:?} instead",
                        span.name
                    ),
                    (SpanOrEvent::Span { span, .. }, Some(Expect::Enter(ref expected_span))) => {
                        if let Some(name) = expected_span.name {
                            assert_eq!(name, span.name);
                        }
                        // TODO: expect fields
                    }
                    (SpanOrEvent::Span { span, .. }, Some(Expect::Exit(ref expected_span))) => {
                        panic!(
                            "expected to exit {}, but entered span {:?} instead",
                            expected_span, span.name
                        )
                    }
                    (
                        SpanOrEvent::Span { span, .. },
                        Some(Expect::CloneSpan(ref expected_span)),
                    ) => panic!(
                        "expected to clone {}, but entered span {:?} instead",
                        expected_span, span.name
                    ),
                    (SpanOrEvent::Span { span, .. }, Some(Expect::DropSpan(ref expected_span))) => {
                        panic!(
                            "expected to drop {}, but entered span {:?} instead",
                            expected_span, span.name
                        )
                    }
                    (SpanOrEvent::Span { span, .. }, Some(Expect::Nothing)) => panic!(
                        "expected nothing else to happen, but entered span {:?}",
                        span.name,
                    ),
                }
            };
        }

        fn exit(&self, id: &Span) {
            let spans = self.spans.lock().unwrap();
            let span = spans
                .get(id)
                .unwrap_or_else(|| panic!("no span for ID {:?}", id));
            println!("exit: {}; id={:?};", span.name(), id);
            match (span, self.expected.lock().unwrap().pop_front()) {
                (_, None) => {}
                (SpanOrEvent::Event, _) => panic!("events should never be exited!"),
                (SpanOrEvent::Span { span, .. }, Some(Expect::Event(_))) => {
                    panic!("expected an event, but exited span {:?} instead", span.name)
                }
                (SpanOrEvent::Span { span, .. }, Some(Expect::Enter(ref expected_span))) => panic!(
                    "expected to enter {}, but exited span {:?} instead",
                    expected_span, span.name
                ),
                (SpanOrEvent::Span { span, .. }, Some(Expect::Exit(ref expected_span))) => {
                    if let Some(name) = expected_span.name {
                        assert_eq!(name, span.name);
                    }
                    // TODO: expect fields
                }
                (SpanOrEvent::Span { span, .. }, Some(Expect::CloneSpan(ref expected_span))) => {
                    panic!(
                        "expected to clone {}, but exited span {:?} instead",
                        expected_span, span.name
                    )
                }
                (SpanOrEvent::Span { span, .. }, Some(Expect::DropSpan(ref expected_span))) => {
                    panic!(
                        "expected to drop {}, but exited span {:?} instead",
                        expected_span, span.name
                    )
                }
                (SpanOrEvent::Span { span, .. }, Some(Expect::Nothing)) => panic!(
                    "expected nothing else to happen, but exited span {:?}",
                    span.name,
                ),
            };
        }

        fn clone_span(&self, id: &Span) -> Span {
            let name = self.spans.lock().unwrap().get_mut(id).map(|span_or_event| {
                if let SpanOrEvent::Span {
                    ref span,
                    ref mut refs,
                } = span_or_event
                {
                    let name = span.name;
                    println!("clone_span: {}; id={:?}; refs={:?};", name, id, *refs);
                    *refs += 1;
                    name
                } else {
                    println!("clone_span: event; id={:?};", id);
                    "event"
                }
            });
            if name.is_none() {
                println!("clone_span: id={:?};", id);
            }
            let mut expected = self.expected.lock().unwrap();
            let was_expected = if let Some(Expect::CloneSpan(ref span)) = expected.front() {
                assert_eq!(name, span.name);
                true
            } else {
                false
            };
            if was_expected {
                expected.pop_front();
            }
            id.clone()
        }

        fn drop_span(&self, id: Span) {
            let mut is_event = false;
            let name = if let Ok(mut spans) = self.spans.try_lock() {
                spans.get_mut(&id).map(|span_or_event| match span_or_event {
                    SpanOrEvent::Span {
                        ref span,
                        ref mut refs,
                    } => {
                        let name = span.name;
                        println!("drop_span: {}; id={:?}; refs={:?};", name, id, *refs);
                        *refs -= 1;
                        name
                    }
                    SpanOrEvent::Event => {
                        println!("drop_span: event; id={:?}", id);
                        is_event = true;
                        "event"
                    }
                })
            } else {
                None
            };
            if name.is_none() {
                println!("drop_span: id={:?}", id);
            }
            if let Ok(mut expected) = self.expected.try_lock() {
                let was_expected = match expected.front() {
                    Some(Expect::DropSpan(ref span)) => {
                        // Don't assert if this function was called while panicking,
                        // as failing the assertion can cause a double panic.
                        if !::std::thread::panicking() {
                            assert_eq!(name, span.name);
                        }
                        true
                    }
                    Some(Expect::Event(_)) => {
                        if !::std::thread::panicking() {
                            assert!(is_event);
                        }
                        true
                    }
                    _ => false,
                };
                if was_expected {
                    expected.pop_front();
                }
            }
        }
    }

    impl MockHandle {
        pub fn assert_finished(&self) {
            if let Ok(ref expected) = self.0.lock() {
                assert!(
                    !expected.iter().any(|thing| thing != &Expect::Nothing),
                    "more notifications expected: {:?}",
                    **expected
                );
            }
        }
    }

}
