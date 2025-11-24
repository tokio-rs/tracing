//! An implementation of the [`Subscriber`] trait to receive and validate
//! `tracing` data.
//!
//! The [`MockSubscriber`] is the central component of this crate. The
//! `MockSubscriber` has expectations set on it which are later
//! validated as the code under test is run.
//!
//! # Examples
//!
//! ```
//! use tracing_mock::{expect, subscriber, field};
//!
//! let (subscriber, handle) = subscriber::mock()
//!     // Expect a single event with a specified message
//!     .event(expect::event().with_fields(expect::msg("droids")))
//!     .only()
//!     .run_with_handle();
//!
//! // Use `with_default` to apply the `MockSubscriber` for the duration
//! // of the closure - this is what we are testing.
//! tracing::subscriber::with_default(subscriber, || {
//!     // These *are* the droids we are looking for
//!     tracing::info!("droids");
//! });
//!
//! // Use the handle to check the assertions. This line will panic if an
//! // assertion is not met.
//! handle.assert_finished();
//! ```
//!
//! A more complex example may consider multiple spans and events with
//! their respective fields:
//!
//! ```
//! use tracing_mock::{expect, subscriber, field};
//!
//! let span = expect::span()
//!     .named("my_span");
//! let (subscriber, handle) = subscriber::mock()
//!     // Enter a matching span
//!     .enter(&span)
//!     // Record an event with message "subscriber parting message"
//!     .event(expect::event().with_fields(expect::msg("subscriber parting message")))
//!     // Record a value for the field `parting` on a matching span
//!     .record(&span, expect::field("parting").with_value(&"goodbye world!"))
//!     // Exit a matching span
//!     .exit(span)
//!     // Expect no further messages to be recorded
//!     .only()
//!     // Return the subscriber and handle
//!     .run_with_handle();
//!
//! // Use `with_default` to apply the `MockSubscriber` for the duration
//! // of the closure - this is what we are testing.
//! tracing::subscriber::with_default(subscriber, || {
//!     let span = tracing::trace_span!(
//!         "my_span",
//!         greeting = "hello world",
//!         parting = tracing::field::Empty
//!     );
//!
//!     let _guard = span.enter();
//!     tracing::info!("subscriber parting message");
//!     let parting = "goodbye world!";
//!
//!     span.record("parting", &parting);
//! });
//!
//! // Use the handle to check the assertions. This line will panic if an
//! // assertion is not met.
//! handle.assert_finished();
//! ```
//!
//! If we modify the previous example so that we **don't** enter the
//! span before recording an event, the test will fail:
//!
//! ```should_panic
//! use tracing_mock::{expect, subscriber, field};
//!
//! let span = expect::span()
//!     .named("my_span");
//! let (subscriber, handle) = subscriber::mock()
//!     .enter(&span)
//!     .event(expect::event().with_fields(expect::msg("collect parting message")))
//!     .record(&span, expect::field("parting").with_value(&"goodbye world!"))
//!     .exit(span)
//!     .only()
//!     .run_with_handle();
//!
//! // Use `with_default` to apply the `MockSubscriber` for the duration
//! // of the closure - this is what we are testing.
//! tracing::subscriber::with_default(subscriber, || {
//!     let span = tracing::trace_span!(
//!         "my_span",
//!         greeting = "hello world",
//!         parting = tracing::field::Empty
//!     );
//!
//!     // Don't enter the span.
//!     // let _guard = span.enter();
//!     tracing::info!("subscriber parting message");
//!     let parting = "goodbye world!";
//!
//!     span.record("parting", &parting);
//! });
//!
//! // Use the handle to check the assertions. This line will panic if an
//! // assertion is not met.
//! handle.assert_finished();
//! ```
//!
//! This will result in an error message such as the following:
//!
//! ```text
//! thread 'main' panicked at '
//! [main] expected to enter a span named `my_span`
//! [main] but instead observed event Event {
//!     fields: ValueSet {
//!         message: subscriber parting message,
//!         callsite: Identifier(0x10eda3278),
//!     },
//!     metadata: Metadata {
//!         name: "event src/subscriber.rs:27",
//!         target: "rust_out",
//!         level: Level(
//!             Info,
//!         ),
//!         module_path: "rust_out",
//!         location: src/subscriber.rs:27,
//!         fields: {message},
//!         callsite: Identifier(0x10eda3278),
//!         kind: Kind(EVENT),
//!     },
//!     parent: Current,
//! }', tracing/tracing-mock/src/expect.rs:59:33
//! ```
//!
//! [`Subscriber`]: trait@tracing::Subscriber
//! [`MockSubscriber`]: struct@crate::subscriber::MockSubscriber
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
};
use tracing::{
    level_filters::LevelFilter,
    span::{self, Attributes, Id},
    subscriber::Interest,
    Event, Metadata, Subscriber,
};

use crate::{
    ancestry::get_ancestry,
    event::ExpectedEvent,
    expect::Expect,
    field::ExpectedFields,
    span::{ActualSpan, ExpectedSpan, NewSpan},
};

pub(crate) struct SpanState {
    id: Id,
    name: &'static str,
    refs: usize,
    meta: &'static Metadata<'static>,
}

impl From<&SpanState> for ActualSpan {
    fn from(span_state: &SpanState) -> Self {
        Self::new(span_state.id.clone(), Some(span_state.meta))
    }
}

struct Running<F: Fn(&Metadata<'_>) -> bool> {
    spans: Mutex<HashMap<Id, SpanState>>,
    expected: Arc<Mutex<VecDeque<Expect>>>,
    current: Mutex<Vec<Id>>,
    ids: AtomicUsize,
    max_level: Option<LevelFilter>,
    filter: F,
    name: String,
}

/// A subscriber which can validate received traces.
///
/// For a detailed description and examples see the documentation
/// for the methods and the [`subscriber`] module.
///
/// [`subscriber`]: mod@crate::subscriber
#[derive(Debug)]
pub struct MockSubscriber<F: Fn(&Metadata<'_>) -> bool> {
    expected: VecDeque<Expect>,
    max_level: Option<LevelFilter>,
    filter: F,
    name: String,
}

/// A handle which is used to invoke validation of expectations.
///
/// The handle is currently only used to assert that all the expected
/// events and spans were seen.
///
/// For additional information and examples, see the [`subscriber`]
/// module documentation.
///
/// [`subscriber`]: mod@crate::subscriber
#[derive(Debug)]
pub struct MockHandle(Arc<Mutex<VecDeque<Expect>>>, String);

/// Create a new [`MockSubscriber`].
///
/// For additional information and examples, see the [`subscriber`]
/// module and [`MockSubscriber`] documentation.
///
/// # Examples
///
///
/// ```
/// use tracing_mock::{expect, subscriber, field};
///
/// let span = expect::span()
///     .named("my_span");
/// let (subscriber, handle) = subscriber::mock()
///     // Enter a matching span
///     .enter(&span)
///     // Record an event with message "subscriber parting message"
///     .event(expect::event().with_fields(expect::msg("subscriber parting message")))
///     // Record a value for the field `parting` on a matching span
///     .record(&span, expect::field("parting").with_value(&"goodbye world!"))
///     // Exit a matching span
///     .exit(span)
///     // Expect no further messages to be recorded
///     .only()
///     // Return the subscriber and handle
///     .run_with_handle();
///
/// // Use `with_default` to apply the `MockSubscriber` for the duration
/// // of the closure - this is what we are testing.
/// tracing::subscriber::with_default(subscriber, || {
///     let span = tracing::trace_span!(
///         "my_span",
///         greeting = "hello world",
///         parting = tracing::field::Empty
///     );
///
///     let _guard = span.enter();
///     tracing::info!("subscriber parting message");
///     let parting = "goodbye world!";
///
///     span.record("parting", &parting);
/// });
///
/// // Use the handle to check the assertions. This line will panic if an
/// // assertion is not met.
/// handle.assert_finished();
/// ```
///
/// [`subscriber`]: mod@crate::subscriber
#[must_use]
pub fn mock() -> MockSubscriber<fn(&Metadata<'_>) -> bool> {
    MockSubscriber {
        expected: VecDeque::new(),
        filter: (|_: &Metadata<'_>| true) as for<'r, 's> fn(&'r Metadata<'s>) -> _,
        max_level: None,
        name: thread::current()
            .name()
            .unwrap_or("mock_subscriber")
            .to_string(),
    }
}

impl<F> MockSubscriber<F>
where
    F: Fn(&Metadata<'_>) -> bool + 'static,
{
    /// Overrides the name printed by the mock subscriber's debugging output.
    ///
    /// The debugging output is displayed if the test panics, or if the test is
    /// run with `--nocapture`.
    ///
    /// By default, the mock subscriber's name is the  name of the test
    /// (*technically*, the name of the thread where it was created, which is
    /// the name of the test unless tests are run with `--test-threads=1`).
    /// When a test has only one mock subscriber, this is sufficient. However,
    /// some tests may include multiple subscribers, in order to test
    /// interactions between multiple subscribers. In that case, it can be
    /// helpful to give each subscriber a separate name to distinguish where the
    /// debugging output comes from.
    ///
    /// # Examples
    ///
    /// In the following example, we create 2 subscribers, both
    /// expecting to receive an event. As we only record a single
    /// event, the test will fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let (subscriber_1, handle_1) = subscriber::mock()
    ///     .named("subscriber-1")
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// let (subscriber_2, handle_2) = subscriber::mock()
    ///     .named("subscriber-2")
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// let _guard = tracing::subscriber::set_default(subscriber_2);
    ///
    /// tracing::subscriber::with_default(subscriber_1, || {
    ///     tracing::info!("a");
    /// });
    ///
    /// handle_1.assert_finished();
    /// handle_2.assert_finished();
    /// ```
    ///
    /// In the test output, we see that the subscriber which didn't
    /// received the event was the one named `subscriber-2`, which is
    /// correct as the subscriber named `subscriber-1` was the default
    /// when the event was recorded:
    ///
    /// ```text
    /// [subscriber-2] more notifications expected: [
    ///     Event(
    ///         MockEvent,
    ///     ),
    /// ]', tracing-mock/src/subscriber.rs:1276:13
    /// ```
    pub fn named(self, name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            ..self
        }
    }

    /// Adds an expectation that an event matching the [`ExpectedEvent`]
    /// will be recorded next.
    ///
    /// The `event` can be a default mock which will match any event
    /// (`expect::event()`) or can include additional expectations.
    /// See the [`ExpectedEvent`] documentation for more details.
    ///
    /// If an event is recorded that doesn't match the `ExpectedEvent`,
    /// or if something else (such as entering a span) is recorded
    /// first, then the expectation will fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let (subscriber, handle) = subscriber::mock()
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     tracing::info!("a");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// A span is entered before the event, causing the test to fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let (subscriber, handle) = subscriber::mock()
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     let span = tracing::info_span!("span");
    ///     let _guard = span.enter();
    ///     tracing::info!("a");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    pub fn event(mut self, event: ExpectedEvent) -> Self {
        self.expected.push_back(Expect::Event(event));
        self
    }

    /// Adds an expectation that the creation of a span will be
    /// recorded next.
    ///
    /// This function accepts `Into<NewSpan>` instead of
    /// [`ExpectedSpan`] directly, so it can be used to test
    /// span fields and the span parent. This is because a
    /// subscriber only receives the span fields and parent when
    /// a span is created, not when it is entered.
    ///
    /// The new span doesn't need to be entered for this expectation
    /// to succeed.
    ///
    /// If a span is recorded that doesn't match the `ExpectedSpan`,
    /// or if something else (such as an event) is recorded first,
    /// then the expectation will fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing")
    ///     .with_fields(expect::field("testing").with_value(&"yes"));
    /// let (subscriber, handle) = subscriber::mock()
    ///     .new_span(span)
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     _ = tracing::info_span!("the span we're testing", testing = "yes");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// An event is recorded before the span is created, causing the
    /// test to fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing")
    ///     .with_fields(expect::field("testing").with_value(&"yes"));
    /// let (subscriber, handle) = subscriber::mock()
    ///     .new_span(span)
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     tracing::info!("an event");
    ///     _ = tracing::info_span!("the span we're testing", testing = "yes");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    pub fn new_span<I>(mut self, new_span: I) -> Self
    where
        I: Into<NewSpan>,
    {
        self.expected.push_back(Expect::NewSpan(new_span.into()));
        self
    }

    /// Adds an expectation that entering a span matching the
    /// [`ExpectedSpan`] will be recorded next.
    ///
    /// This expectation is generally accompanied by a call to
    /// [`exit`] as well. If used together with [`only`], this
    /// is necessary.
    ///
    /// If the span that is entered doesn't match the [`ExpectedSpan`],
    /// or if something else (such as an event) is recorded first,
    /// then the expectation will fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing");
    /// let (subscriber, handle) = subscriber::mock()
    ///     .enter(&span)
    ///     .exit(&span)
    ///     .only()
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     let span = tracing::info_span!("the span we're testing");
    ///     let _entered = span.enter();
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// An event is recorded before the span is entered, causing the
    /// test to fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing");
    /// let (subscriber, handle) = subscriber::mock()
    ///     .enter(&span)
    ///     .exit(&span)
    ///     .only()
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     tracing::info!("an event");
    ///     let span = tracing::info_span!("the span we're testing");
    ///     let _entered = span.enter();
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`exit`]: fn@Self::exit
    /// [`only`]: fn@Self::only
    pub fn enter<S>(mut self, span: S) -> Self
    where
        S: Into<ExpectedSpan>,
    {
        self.expected.push_back(Expect::Enter(span.into()));
        self
    }

    /// Adds ab expectation that exiting a span matching the
    /// [`ExpectedSpan`] will be recorded next.
    ///
    /// As a span may be entered and exited multiple times,
    /// this is different from the span being closed. In
    /// general [`enter`] and `exit` should be paired.
    ///
    /// If the span that is exited doesn't match the [`ExpectedSpan`],
    /// or if something else (such as an event) is recorded first,
    /// then the expectation will fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing");
    /// let (subscriber, handle) = subscriber::mock()
    ///     .enter(&span)
    ///     .exit(&span)
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     let span = tracing::info_span!("the span we're testing");
    ///     let _entered = span.enter();
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// An event is recorded before the span is exited, causing the
    /// test to fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing");
    /// let (subscriber, handle) = subscriber::mock()
    ///     .enter(&span)
    ///     .exit(&span)
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     let span = tracing::info_span!("the span we're testing");
    ///     let _entered = span.enter();
    ///     tracing::info!("an event");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`enter`]: fn@Self::enter
    pub fn exit<S>(mut self, span: S) -> Self
    where
        S: Into<ExpectedSpan>,
    {
        self.expected.push_back(Expect::Exit(span.into()));
        self
    }

    /// Adds an expectation that cloning a span matching the
    /// [`ExpectedSpan`] will be recorded next.
    ///
    /// The cloned span does need to be entered.
    ///
    /// If the span that is cloned doesn't match the [`ExpectedSpan`],
    /// or if something else (such as an event) is recorded first,
    /// then the expectation will fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing");
    /// let (subscriber, handle) = subscriber::mock()
    ///     .clone_span(span)
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     let span = tracing::info_span!("the span we're testing");
    ///     _ = span.clone();
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// An event is recorded before the span is cloned, causing the
    /// test to fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing");
    /// let (subscriber, handle) = subscriber::mock()
    ///     .clone_span(span)
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     let span = tracing::info_span!("the span we're testing");
    ///     tracing::info!("an event");
    ///     _ = span.clone();
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    pub fn clone_span<S>(mut self, span: S) -> Self
    where
        S: Into<ExpectedSpan>,
    {
        self.expected.push_back(Expect::CloneSpan(span.into()));
        self
    }

    /// **This method is deprecated.**
    ///
    /// Adds an expectation that a span matching the [`ExpectedSpan`]
    /// getting dropped via the deprecated function
    /// [`Subscriber::drop_span`] will be recorded next.
    ///
    /// Instead [`Subscriber::try_close`] should be used on the subscriber
    /// and should be asserted with `close_span` (which hasn't been
    /// implemented yet, but will be done as part of #539).
    ///
    /// [`Subscriber::drop_span`]: fn@tracing::Subscriber::drop_span
    #[allow(deprecated)]
    pub fn drop_span<S>(mut self, span: S) -> Self
    where
        S: Into<ExpectedSpan>,
    {
        self.expected.push_back(Expect::DropSpan(span.into()));
        self
    }

    /// Adds an expectation that a `follows_from` relationship will be
    /// recorded next. Specifically that a span matching `consequence`
    /// follows from a span matching `cause`.
    ///
    /// For further details on what this causal relationship means, see
    /// [`Span::follows_from`].
    ///
    /// If either of the 2 spans don't match their respective
    /// [`ExpectedSpan`] or if something else (such as an event) is
    /// recorded first, then the expectation will fail.
    ///
    /// **Note**: The 2 spans, `consequence` and `cause` are matched
    /// by `name` only.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let cause = expect::span().named("cause");
    /// let consequence = expect::span().named("consequence");
    ///
    /// let (subscriber, handle) = subscriber::mock()
    ///     .follows_from(consequence, cause)
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     let cause = tracing::info_span!("cause");
    ///     let consequence = tracing::info_span!("consequence");
    ///
    ///     consequence.follows_from(&cause);
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// The `cause` span doesn't match, it is actually recorded at
    /// `Level::WARN` instead of the expected `Level::INFO`, causing
    /// this test to fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let cause = expect::span().named("cause");
    /// let consequence = expect::span().named("consequence");
    ///
    /// let (subscriber, handle) = subscriber::mock()
    ///     .follows_from(consequence, cause)
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     let cause = tracing::info_span!("another cause");
    ///     let consequence = tracing::info_span!("consequence");
    ///
    ///     consequence.follows_from(&cause);
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`Span::follows_from`]: fn@tracing::Span::follows_from
    pub fn follows_from<S1, S2>(mut self, consequence: S1, cause: S2) -> Self
    where
        S1: Into<ExpectedSpan>,
        S2: Into<ExpectedSpan>,
    {
        self.expected.push_back(Expect::FollowsFrom {
            consequence: consequence.into(),
            cause: cause.into(),
        });
        self
    }

    /// Adds an expectation that `fields` are recorded on a span
    /// matching the [`ExpectedSpan`] will be recorded next.
    ///
    /// For further information on how to specify the expected
    /// fields, see the documentation on the [`field`] module.
    ///
    /// If either the span doesn't match the [`ExpectedSpan`], the
    /// fields don't match the expected fields, or if something else
    /// (such as an event) is recorded first, then the expectation
    /// will fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let span = expect::span()
    ///     .named("my_span");
    /// let (subscriber, handle) = subscriber::mock()
    ///     .record(span, expect::field("parting").with_value(&"goodbye world!"))
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     let span = tracing::trace_span!(
    ///         "my_span",
    ///         greeting = "hello world",
    ///         parting = tracing::field::Empty
    ///     );
    ///     span.record("parting", "goodbye world!");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// The value of the recorded field doesn't match the expectation,
    /// causing the test to fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let span = expect::span()
    ///     .named("my_span");
    /// let (subscriber, handle) = subscriber::mock()
    ///     .record(span, expect::field("parting").with_value(&"goodbye world!"))
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     let span = tracing::trace_span!(
    ///         "my_span",
    ///         greeting = "hello world",
    ///         parting = tracing::field::Empty
    ///     );
    ///     span.record("parting", "goodbye universe!");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`field`]: mod@crate::field
    pub fn record<S, I>(mut self, span: S, fields: I) -> Self
    where
        S: Into<ExpectedSpan>,
        I: Into<ExpectedFields>,
    {
        self.expected
            .push_back(Expect::Visit(span.into(), fields.into()));
        self
    }

    /// Adds an expectation that [`Subscriber::on_register_dispatch`] will
    /// be called next.
    ///
    /// **Note**: This expectation is usually fulfilled automatically when
    /// a subscriber is set as the default via [`tracing::subscriber::with_default`]
    /// or [`tracing::subscriber::set_global_default`], so explicitly expecting
    /// this is not usually necessary. However, it may be useful when testing
    /// custom subscriber implementations that manually call `on_register_dispatch`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let (subscriber, handle) = subscriber::mock()
    ///     .on_register_dispatch()
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     // The subscriber's on_register_dispatch was called when it was set as default
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    ///
    /// ```should_panic
    /// use tracing_mock::{subscriber};
    ///
    /// struct WrapSubscriber<S: tracing::Subscriber> {
    ///     inner: S,
    /// }
    ///
    /// impl<S: tracing::Subscriber> tracing::Subscriber for WrapSubscriber<S> {
    /// #     fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
    /// #         self.inner.enabled(metadata)
    /// #     }
    /// #     fn new_span(&self, span: &tracing_core::span::Attributes<'_>) -> tracing_core::span::Id {
    /// #         self.inner.new_span(span)
    /// #     }
    /// #     fn record(&self, span: &tracing_core::span::Id, values: &tracing_core::span::Record<'_>) {
    /// #         self.inner.record(span, values)
    /// #     }
    /// #     fn record_follows_from(
    /// #         &self,
    /// #         span: &tracing_core::span::Id,
    /// #         follows: &tracing_core::span::Id,
    /// #     ) {
    /// #         self.inner.record_follows_from(span, follows)
    /// #     }
    /// #     fn event(&self, event: &tracing::Event<'_>) {
    /// #         self.inner.event(event)
    /// #     }
    /// #     fn enter(&self, span: &tracing_core::span::Id) {
    /// #         self.inner.enter(span)
    /// #     }
    /// #     fn exit(&self, span: &tracing_core::span::Id) {
    /// #         self.inner.exit(span)
    /// #     }
    ///     // All other Subscriber methods implemented to forward correctly.
    ///
    ///     fn on_register_dispatch(&self, subscriber: &tracing::Dispatch) {
    ///         // Doesn't forward to `self.inner`
    ///         let _ = subscriber;
    ///     }
    /// }
    ///
    /// let (subscriber, handle) = subscriber::mock().on_register_dispatch().run_with_handle();
    /// let wrap_subscriber = WrapSubscriber { inner: subscriber };
    ///
    /// tracing::subscriber::with_default(wrap_subscriber, || {
    ///     // The subscriber's on_register_dispatch is called when set as default
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`Subscriber::on_register_dispatch`]: tracing::Subscriber::on_register_dispatch
    pub fn on_register_dispatch(mut self) -> Self {
        self.expected.push_back(Expect::OnRegisterDispatch);
        self
    }

    /// Filter the traces evaluated by the `MockSubscriber`.
    ///
    /// The filter will be applied to all traces received before
    /// any validation occurs - so its position in the call chain
    /// is not important. The filter does not perform any validation
    /// itself.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let (subscriber, handle) = subscriber::mock()
    ///     .with_filter(|meta| meta.level() <= &tracing::Level::WARN)
    ///     .event(expect::event())
    ///     .only()
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     tracing::info!("a");
    ///     tracing::warn!("b");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    pub fn with_filter<G>(self, filter: G) -> MockSubscriber<G>
    where
        G: Fn(&Metadata<'_>) -> bool + 'static,
    {
        MockSubscriber {
            expected: self.expected,
            filter,
            max_level: self.max_level,
            name: self.name,
        }
    }

    /// Sets the max level that will be provided to the `tracing`
    /// system.
    ///
    /// This method can be used to test the internals of `tracing`,
    /// but it is also useful to filter out traces on more verbose
    /// levels if you only want to verify above a certain level.
    ///
    /// **Note**: this value determines a global filter, if
    /// `with_max_level_hint` is called on multiple subscribers, the
    /// global filter will be the least restrictive of all subscribers.
    /// To filter the events evaluated by a specific `MockSubscriber`,
    /// use [`with_filter`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let (subscriber, handle) = subscriber::mock()
    ///     .with_max_level_hint(tracing::Level::INFO)
    ///     .event(expect::event().at_level(tracing::Level::INFO))
    ///     .only()
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     tracing::debug!("a message we don't care about");
    ///     tracing::info!("a message we want to validate");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`with_filter`]: fn@Self::with_filter
    pub fn with_max_level_hint(self, hint: impl Into<LevelFilter>) -> Self {
        Self {
            max_level: Some(hint.into()),
            ..self
        }
    }

    /// Expects that no further traces are received.
    ///
    /// The call to `only` should appear immediately before the final
    /// call to `run` or `run_with_handle`, as any expectations which
    /// are added after `only` will not be considered.
    ///
    /// # Examples
    ///
    /// Consider this simple test. It passes even though we only
    /// expect a single event, but receive three:
    ///
    /// ```
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let (subscriber, handle) = subscriber::mock()
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     tracing::info!("a");
    ///     tracing::info!("b");
    ///     tracing::info!("c");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// After including `only`, the test will fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let (subscriber, handle) = subscriber::mock()
    ///     .event(expect::event())
    ///     .only()
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     tracing::info!("a");
    ///     tracing::info!("b");
    ///     tracing::info!("c");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    pub fn only(mut self) -> Self {
        self.expected.push_back(Expect::Nothing);
        self
    }

    /// Consume the receiver and return an `impl` [`Subscriber`] which can
    /// be set as the default subscriber.
    ///
    /// This function is similar to [`run_with_handle`], but it doesn't
    /// return a [`MockHandle`]. This is useful if the desired
    /// assertions can be checked externally to the subscriber.
    ///
    /// # Examples
    ///
    /// The following test is used within the `tracing`
    /// codebase:
    ///
    /// ```
    /// use tracing_mock::subscriber;
    ///
    /// tracing::subscriber::with_default(subscriber::mock().run(), || {
    ///     let foo1 = tracing::span!(tracing::Level::TRACE, "foo");
    ///     let foo2 = foo1.clone();
    ///     // Two handles that point to the same span are equal.
    ///     assert_eq!(foo1, foo2);
    /// });
    /// ```
    ///
    /// [`Subscriber`]: tracing::Subscriber
    /// [`run_with_handle`]: fn@Self::run_with_handle
    pub fn run(self) -> impl Subscriber {
        let (subscriber, _) = self.run_with_handle();
        subscriber
    }

    /// Consume the receiver and return an `impl` [`Subscriber`] which can
    /// be set as the default subscriber and a [`MockHandle`] which can
    /// be used to validate the provided expectations.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{expect, subscriber};
    ///
    /// // subscriber and handle are returned from `run_with_handle()`
    /// let (subscriber, handle) = subscriber::mock()
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     tracing::info!("a");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`Subscriber`]: tracing::Subscriber
    pub fn run_with_handle(self) -> (impl Subscriber, MockHandle) {
        let expected = Arc::new(Mutex::new(self.expected));
        let handle = MockHandle(expected.clone(), self.name.clone());
        let subscriber = Running {
            spans: Mutex::new(HashMap::new()),
            expected,
            current: Mutex::new(Vec::new()),
            ids: AtomicUsize::new(1),
            filter: self.filter,
            max_level: self.max_level,
            name: self.name,
        };
        (subscriber, handle)
    }
}

impl<F> Subscriber for Running<F>
where
    F: Fn(&Metadata<'_>) -> bool + 'static,
{
    fn on_register_dispatch(&self, _subscriber: &tracing::Dispatch) {
        println!("[{}] on_register_dispatch", self.name);
        let mut expected = self.expected.lock().unwrap();
        if let Some(Expect::OnRegisterDispatch) = expected.front() {
            expected.pop_front();
        }
    }

    fn enabled(&self, meta: &Metadata<'_>) -> bool {
        println!("[{}] enabled: {:#?}", self.name, meta);
        let enabled = (self.filter)(meta);
        println!("[{}] enabled -> {}", self.name, enabled);
        enabled
    }

    fn register_callsite(&self, meta: &'static Metadata<'static>) -> Interest {
        println!("[{}] register_callsite: {:#?}", self.name, meta);
        if self.enabled(meta) {
            Interest::always()
        } else {
            Interest::never()
        }
    }
    fn max_level_hint(&self) -> Option<LevelFilter> {
        self.max_level
    }

    fn record(&self, id: &Id, values: &span::Record<'_>) {
        let spans = self.spans.lock().unwrap();
        let mut expected = self.expected.lock().unwrap();
        let span = spans
            .get(id)
            .unwrap_or_else(|| panic!("[{}] no span for ID {:?}", self.name, id));
        println!(
            "[{}] record: {}; id={:?}; values={:?};",
            self.name, span.name, id, values
        );
        let was_expected = matches!(expected.front(), Some(Expect::Visit(_, _)));
        if was_expected {
            if let Expect::Visit(expected_span, mut expected_values) = expected.pop_front().unwrap()
            {
                if let Some(name) = expected_span.name() {
                    assert_eq!(name, span.name);
                }
                let context = format!("span {}: ", span.name);
                let mut checker = expected_values.checker(&context, &self.name);
                values.record(&mut checker);
                checker.finish();
            }
        }
    }

    fn event(&self, event: &Event<'_>) {
        let name = event.metadata().name();
        println!("[{}] event: {};", self.name, name);
        match self.expected.lock().unwrap().pop_front() {
            None => {}
            Some(Expect::Event(mut expected)) => {
                #[cfg(feature = "tracing-subscriber")]
                {
                    if expected.scope_mut().is_some() {
                        unimplemented!(
                            "Expected scope for events is not supported with `MockSubscriber`."
                        )
                    }
                }
                let event_get_ancestry = || {
                    get_ancestry(
                        event,
                        || self.lookup_current(),
                        |span_id| {
                            self.spans
                                .lock()
                                .unwrap()
                                .get(span_id)
                                .map(|span| span.into())
                        },
                    )
                };
                expected.check(event, event_get_ancestry, &self.name);
            }
            Some(ex) => ex.bad(&self.name, format_args!("observed event {:#?}", event)),
        }
    }

    fn record_follows_from(&self, consequence_id: &Id, cause_id: &Id) {
        let spans = self.spans.lock().unwrap();
        if let Some(consequence_span) = spans.get(consequence_id) {
            if let Some(cause_span) = spans.get(cause_id) {
                println!(
                    "[{}] record_follows_from: {} (id={:?}) follows {} (id={:?})",
                    self.name, consequence_span.name, consequence_id, cause_span.name, cause_id,
                );
                match self.expected.lock().unwrap().pop_front() {
                    None => {}
                    Some(Expect::FollowsFrom {
                        consequence: ref expected_consequence,
                        cause: ref expected_cause,
                    }) => {
                        if let Some(name) = expected_consequence.name() {
                            // TODO(hds): Write proper assertion text.
                            assert_eq!(name, consequence_span.name);
                        }
                        if let Some(name) = expected_cause.name() {
                            // TODO(hds): Write proper assertion text.
                            assert_eq!(name, cause_span.name);
                        }
                    }
                    Some(ex) => ex.bad(
                        &self.name,
                        format_args!(
                            "consequence {:?} followed cause {:?}",
                            consequence_span.name, cause_span.name
                        ),
                    ),
                }
            }
        };
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let meta = span.metadata();
        let id = self.ids.fetch_add(1, Ordering::SeqCst);
        let id = Id::from_u64(id as u64);
        println!(
            "[{}] new_span: name={:?}; target={:?}; id={:?};",
            self.name,
            meta.name(),
            meta.target(),
            id
        );
        let mut expected = self.expected.lock().unwrap();
        let was_expected = matches!(expected.front(), Some(Expect::NewSpan(_)));
        let mut spans = self.spans.lock().unwrap();
        if was_expected {
            if let Expect::NewSpan(mut expected) = expected.pop_front().unwrap() {
                if let Some(expected_id) = &expected.span.id {
                    expected_id.set(id.into_u64()).unwrap();
                }

                expected.check(
                    span,
                    || {
                        get_ancestry(
                            span,
                            || self.lookup_current(),
                            |span_id| spans.get(span_id).map(|span| span.into()),
                        )
                    },
                    &self.name,
                );
            }
        }
        spans.insert(
            id.clone(),
            SpanState {
                id: id.clone(),
                name: meta.name(),
                refs: 1,
                meta,
            },
        );
        id
    }

    fn enter(&self, id: &Id) {
        let spans = self.spans.lock().unwrap();
        if let Some(span) = spans.get(id) {
            println!("[{}] enter: {}; id={:?};", self.name, span.name, id);
            match self.expected.lock().unwrap().pop_front() {
                None => {}
                Some(Expect::Enter(ref expected_span)) => {
                    expected_span.check(&span.into(), "to enter a span", &self.name);
                }
                Some(ex) => ex.bad(&self.name, format_args!("entered span {:?}", span.name)),
            }
        };
        self.current.lock().unwrap().push(id.clone());
    }

    fn exit(&self, id: &Id) {
        if std::thread::panicking() {
            // `exit()` can be called in `drop` impls, so we must guard against
            // double panics.
            println!("[{}] exit {:?} while panicking", self.name, id);
            return;
        }
        let spans = self.spans.lock().unwrap();
        let span = spans
            .get(id)
            .unwrap_or_else(|| panic!("[{}] no span for ID {:?}", self.name, id));
        println!("[{}] exit: {}; id={:?};", self.name, span.name, id);
        match self.expected.lock().unwrap().pop_front() {
            None => {}
            Some(Expect::Exit(ref expected_span)) => {
                expected_span.check(&span.into(), "to exit a span", &self.name);
                let curr = self.current.lock().unwrap().pop();
                assert_eq!(
                    Some(id),
                    curr.as_ref(),
                    "[{}] exited span {:?}, but the current span was {:?}",
                    self.name,
                    span.name,
                    curr.as_ref().and_then(|id| spans.get(id)).map(|s| s.name)
                );
            }
            Some(ex) => ex.bad(&self.name, format_args!("exited span {:?}", span.name)),
        };
    }

    fn clone_span(&self, id: &Id) -> Id {
        let mut spans = self.spans.lock().unwrap();
        let mut span = spans.get_mut(id);
        match span.as_deref_mut() {
            Some(span) => {
                println!(
                    "[{}] clone_span: {}; id={:?}; refs={:?};",
                    self.name, span.name, id, span.refs,
                );
                span.refs += 1;
            }
            None => {
                println!(
                    "[{}] clone_span: id={:?} (not found in span list);",
                    self.name, id
                );
            }
        }

        let mut expected = self.expected.lock().unwrap();
        let was_expected = if let Some(Expect::CloneSpan(ref expected_span)) = expected.front() {
            match span {
                Some(actual_span) => {
                    let actual_span: &_ = actual_span;
                    expected_span.check(&actual_span.into(), "to clone a span", &self.name);
                }
                // Check only by Id
                None => expected_span.check(&id.into(), "to clone a span", &self.name),
            }
            true
        } else {
            false
        };
        if was_expected {
            expected.pop_front();
        }
        id.clone()
    }

    fn drop_span(&self, id: Id) {
        let mut is_event = false;
        let name = if let Ok(mut spans) = self.spans.try_lock() {
            spans.get_mut(&id).map(|span| {
                let name = span.name;
                if name.contains("event") {
                    is_event = true;
                }
                println!(
                    "[{}] drop_span: {}; id={:?}; refs={:?};",
                    self.name, name, id, span.refs
                );
                span.refs -= 1;
                name
            })
        } else {
            None
        };
        if name.is_none() {
            println!("[{}] drop_span: id={:?}", self.name, id);
        }
        if let Ok(mut expected) = self.expected.try_lock() {
            let was_expected = match expected.front() {
                Some(Expect::DropSpan(ref span)) => {
                    // Don't assert if this function was called while panicking,
                    // as failing the assertion can cause a double panic.
                    if !::std::thread::panicking() {
                        assert_eq!(name, span.name());
                    }
                    true
                }
                Some(Expect::Event(_)) => {
                    if !::std::thread::panicking() {
                        assert!(is_event, "[{}] expected an event", self.name);
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

    fn current_span(&self) -> tracing_core::span::Current {
        let stack = self.current.lock().unwrap();
        match stack.last() {
            Some(id) => {
                let spans = self.spans.lock().unwrap();
                let state = spans.get(id).expect("state for current span");
                tracing_core::span::Current::new(id.clone(), state.meta)
            }
            None => tracing_core::span::Current::none(),
        }
    }
}

impl<F> Running<F>
where
    F: Fn(&Metadata<'_>) -> bool,
{
    fn lookup_current(&self) -> Option<span::Id> {
        let stack = self.current.lock().unwrap();
        stack.last().cloned()
    }
}

impl MockHandle {
    #[cfg(feature = "tracing-subscriber")]
    pub(crate) fn new(expected: Arc<Mutex<VecDeque<Expect>>>, name: String) -> Self {
        Self(expected, name)
    }

    /// Checks the expectations which were set on the
    /// [`MockSubscriber`].
    ///
    /// Calling `assert_finished` is usually the final part of a test.
    ///
    /// # Panics
    ///
    /// This method will panic if any of the provided expectations are
    /// not met.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{expect, subscriber};
    ///
    /// let (subscriber, handle) = subscriber::mock()
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// tracing::subscriber::with_default(subscriber, || {
    ///     tracing::info!("a");
    /// });
    ///
    /// // Check assertions set on the mock subscriber
    /// handle.assert_finished();
    /// ```
    pub fn assert_finished(&self) {
        if let Ok(ref expected) = self.0.lock() {
            assert!(
                !expected.iter().any(|thing| thing != &Expect::Nothing),
                "\n[{}] more notifications expected: {:#?}",
                self.1,
                **expected
            );
        }
    }
}
