//! An implementation of the [`Collect`] trait to receive and validate
//! `tracing` data.
//!
//! The [`MockCollector`] is the central component of this crate. The
//! `MockCollector` has expectations set on it which are later
//! validated as the code under test is run.
//!
//! # Examples
//!
//! ```
//! use tracing_mock::{collector, expect, field};
//!
//! let (collector, handle) = collector::mock()
//!     // Expect a single event with a specified message
//!     .event(expect::event().with_fields(expect::message("droids")))
//!     .only()
//!     .run_with_handle();
//!
//! // Use `with_default` to apply the `MockCollector` for the duration
//! // of the closure - this is what we are testing.
//! tracing::collect::with_default(collector, || {
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
//! use tracing_mock::{collector, expect, field};
//!
//! let span = expect::span()
//!     .named("my_span");
//! let (collector, handle) = collector::mock()
//!     // Enter a matching span
//!     .enter(span.clone())
//!     // Record an event with message "collect parting message"
//!     .event(expect::event().with_fields(expect::message("collect parting message")))
//!     // Record a value for the field `parting` on a matching span
//!     .record(span.clone(), expect::field("parting").with_value(&"goodbye world!"))
//!     // Exit a matching span
//!     .exit(span)
//!     // Expect no further messages to be recorded
//!     .only()
//!     // Return the collector and handle
//!     .run_with_handle();
//!
//! // Use `with_default` to apply the `MockCollector` for the duration
//! // of the closure - this is what we are testing.
//! tracing::collect::with_default(collector, || {
//!     let span = tracing::trace_span!(
//!         "my_span",
//!         greeting = "hello world",
//!         parting = tracing::field::Empty
//!     );
//!
//!     let _guard = span.enter();
//!     tracing::info!("collect parting message");
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
//! use tracing_mock::{collector, expect, field};
//!
//! let span = expect::span()
//!     .named("my_span");
//! let (collector, handle) = collector::mock()
//!     .enter(span.clone())
//!     .event(expect::event().with_fields(expect::message("collect parting message")))
//!     .record(span.clone(), expect::field("parting").with_value(&"goodbye world!"))
//!     .exit(span)
//!     .only()
//!     .run_with_handle();
//!
//! // Use `with_default` to apply the `MockCollector` for the duration
//! // of the closure - this is what we are testing.
//! tracing::collect::with_default(collector, || {
//!     let span = tracing::trace_span!(
//!         "my_span",
//!         greeting = "hello world",
//!         parting = tracing::field::Empty
//!     );
//!
//!     // Don't enter the span.
//!     // let _guard = span.enter();
//!     tracing::info!("collect parting message");
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
//!         message: collect parting message,
//!         callsite: Identifier(0x10eda3278),
//!     },
//!     metadata: Metadata {
//!         name: "event src/collector.rs:27",
//!         target: "rust_out",
//!         level: Level(
//!             Info,
//!         ),
//!         module_path: "rust_out",
//!         location: src/collector.rs:27,
//!         fields: {message},
//!         callsite: Identifier(0x10eda3278),
//!         kind: Kind(EVENT),
//!     },
//!     parent: Current,
//! }', tracing/tracing-mock/src/expect.rs:59:33
//! ```
//!
//! [`Collect`]: trait@tracing::Collect
//! [`MockCollector`]: struct@crate::collector::MockCollector
use crate::{
    event::ExpectedEvent,
    expect::Expect,
    field::ExpectedFields,
    span::{ExpectedSpan, NewSpan},
};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
};
use tracing::{
    collect::Interest,
    level_filters::LevelFilter,
    span::{self, Attributes, Id},
    Collect, Event, Metadata,
};

pub(crate) struct SpanState {
    name: &'static str,
    refs: usize,
    meta: &'static Metadata<'static>,
}

impl SpanState {
    pub(crate) fn metadata(&self) -> &'static Metadata<'static> {
        self.meta
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

/// A collector which can validate received traces.
///
/// For a detailed description and examples see the documentation
/// for the methods and the [`collector`] module.
///
/// [`collector`]: mod@crate::collector
pub struct MockCollector<F: Fn(&Metadata<'_>) -> bool> {
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
/// For additional information and examples, see the [`collector`]
/// module documentation.
///
/// [`collector`]: mod@crate::collector
pub struct MockHandle(Arc<Mutex<VecDeque<Expect>>>, String);

/// Create a new [`MockCollector`].
///
/// For additional information and examples, see the [`collector`]
/// module and [`MockCollector`] documentation.
///
/// # Examples
///
///
/// ```
/// use tracing_mock::{collector, expect, field};
///
/// let span = expect::span()
///     .named("my_span");
/// let (collector, handle) = collector::mock()
///     // Enter a matching span
///     .enter(span.clone())
///     // Record an event with message "collect parting message"
///     .event(expect::event().with_fields(expect::message("collect parting message")))
///     // Record a value for the field `parting` on a matching span
///     .record(span.clone(), expect::field("parting").with_value(&"goodbye world!"))
///     // Exit a matching span
///     .exit(span)
///     // Expect no further messages to be recorded
///     .only()
///     // Return the collector and handle
///     .run_with_handle();
///
/// // Use `with_default` to apply the `MockCollector` for the duration
/// // of the closure - this is what we are testing.
/// tracing::collect::with_default(collector, || {
///     let span = tracing::trace_span!(
///         "my_span",
///         greeting = "hello world",
///         parting = tracing::field::Empty
///     );
///
///     let _guard = span.enter();
///     tracing::info!("collect parting message");
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
/// [`collector`]: mod@crate::collector
#[must_use]
pub fn mock() -> MockCollector<fn(&Metadata<'_>) -> bool> {
    MockCollector {
        expected: VecDeque::new(),
        filter: (|_: &Metadata<'_>| true) as for<'r, 's> fn(&'r Metadata<'s>) -> _,
        max_level: None,
        name: thread::current()
            .name()
            .unwrap_or("mock_subscriber")
            .to_string(),
    }
}

impl<F> MockCollector<F>
where
    F: Fn(&Metadata<'_>) -> bool + 'static,
{
    /// Overrides the name printed by the mock subscriber's debugging output.
    ///
    /// The debugging output is displayed if the test panics, or if the test is
    /// run with `--nocapture`.
    ///
    /// By default, the mock collector's name is the  name of the test
    /// (*technically*, the name of the thread where it was created, which is
    /// the name of the test unless tests are run with `--test-threads=1`).
    /// When a test has only one mock collector, this is sufficient. However,
    /// some tests may include multiple collectors, in order to test
    /// interactions between multiple collectors. In that case, it can be
    /// helpful to give each collector a separate name to distinguish where the
    /// debugging output comes from.
    ///
    /// # Examples
    ///
    /// In the following example, we create 2 collectors, both
    /// expecting to receive an event. As we only record a single
    /// event, the test will fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{collector, expect};
    ///
    /// let (collector_1, handle_1) = collector::mock()
    ///     .named("collector-1")
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// let (collector_2, handle_2) = collector::mock()
    ///     .named("collector-2")
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// let _guard = tracing::collect::set_default(collector_2);
    ///
    /// tracing::collect::with_default(collector_1, || {
    ///     tracing::info!("a");
    /// });
    ///
    /// handle_1.assert_finished();
    /// handle_2.assert_finished();
    /// ```
    ///
    /// In the test output, we see that the collector which didn't
    /// received the event was the one named `collector-2`, which is
    /// correct as the collector named `collector-1` was the default
    /// when the event was recorded:
    ///
    /// ```text
    /// [collector-2] more notifications expected: [
    ///     Event(
    ///         MockEvent,
    ///     ),
    /// ]', tracing-mock/src/collector.rs:1276:13
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
    /// use tracing_mock::{collector, expect};
    ///
    /// let (collector, handle) = collector::mock()
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     tracing::info!("a");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// A span is entered before the event, causing the test to fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{collector, expect};
    ///
    /// let (collector, handle) = collector::mock()
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
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
    /// collector only receives the span fields and parent when
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
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing")
    ///     .with_fields(expect::field("testing").with_value(&"yes"));
    /// let (collector, handle) = collector::mock()
    ///     .new_span(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
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
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing")
    ///     .with_fields(expect::field("testing").with_value(&"yes"));
    /// let (collector, handle) = collector::mock()
    ///     .new_span(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
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
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing");
    /// let (collector, handle) = collector::mock()
    ///     .enter(span.clone())
    ///     .exit(span)
    ///     .only()
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
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
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing");
    /// let (collector, handle) = collector::mock()
    ///     .enter(span.clone())
    ///     .exit(span)
    ///     .only()
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
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
    pub fn enter(mut self, span: ExpectedSpan) -> Self {
        self.expected.push_back(Expect::Enter(span));
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
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing");
    /// let (collector, handle) = collector::mock()
    ///     .enter(span.clone())
    ///     .exit(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
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
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing");
    /// let (collector, handle) = collector::mock()
    ///     .enter(span.clone())
    ///     .exit(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     let span = tracing::info_span!("the span we're testing");
    ///     let _entered = span.enter();
    ///     tracing::info!("an event");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`enter`]: fn@Self::enter
    pub fn exit(mut self, span: ExpectedSpan) -> Self {
        self.expected.push_back(Expect::Exit(span));
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
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing");
    /// let (collector, handle) = collector::mock()
    ///     .clone_span(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
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
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing");
    /// let (collector, handle) = collector::mock()
    ///     .clone_span(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     let span = tracing::info_span!("the span we're testing");
    ///     tracing::info!("an event");
    ///     _ = span.clone();
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    pub fn clone_span(mut self, span: ExpectedSpan) -> Self {
        self.expected.push_back(Expect::CloneSpan(span));
        self
    }

    /// **This method is deprecated.**
    ///
    /// Adds an expectation that a span matching the [`ExpectedSpan`]
    /// getting dropped via the deprecated function
    /// [`Collect::drop_span`] will be recorded next.
    ///
    /// Instead [`Collect::try_close`] should be used on the collector
    /// and should be asserted with `close_span` (which hasn't been
    /// implemented yet, but will be done as part of #539).
    ///
    /// [`Collect::drop_span`]: fn@tracing::Collect::drop_span
    #[allow(deprecated)]
    pub fn drop_span(mut self, span: ExpectedSpan) -> Self {
        self.expected.push_back(Expect::DropSpan(span));
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
    /// use tracing_mock::{collector, expect};
    ///
    /// let cause = expect::span().named("cause");
    /// let consequence = expect::span().named("consequence");
    ///
    /// let (collector, handle) = collector::mock()
    ///     .follows_from(consequence, cause)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
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
    /// use tracing_mock::{collector, expect};
    ///
    /// let cause = expect::span().named("cause");
    /// let consequence = expect::span().named("consequence");
    ///
    /// let (collector, handle) = collector::mock()
    ///     .follows_from(consequence, cause)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
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
    pub fn follows_from(mut self, consequence: ExpectedSpan, cause: ExpectedSpan) -> Self {
        self.expected
            .push_back(Expect::FollowsFrom { consequence, cause });
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
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .named("my_span");
    /// let (collector, handle) = collector::mock()
    ///     .record(span, expect::field("parting").with_value(&"goodbye world!"))
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
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
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .named("my_span");
    /// let (collector, handle) = collector::mock()
    ///     .record(span, expect::field("parting").with_value(&"goodbye world!"))
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
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
    pub fn record<I>(mut self, span: ExpectedSpan, fields: I) -> Self
    where
        I: Into<ExpectedFields>,
    {
        self.expected.push_back(Expect::Visit(span, fields.into()));
        self
    }

    /// Filter the traces evaluated by the `MockCollector`.
    ///
    /// The filter will be applied to all traces received before
    /// any validation occurs - so its position in the call chain
    /// is not important. The filter does not perform any validation
    /// itself.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{collector, expect};
    ///
    /// let (collector, handle) = collector::mock()
    ///     .with_filter(|meta| meta.level() <= &tracing::Level::WARN)
    ///     .event(expect::event())
    ///     .only()
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     tracing::info!("a");
    ///     tracing::warn!("b");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    pub fn with_filter<G>(self, filter: G) -> MockCollector<G>
    where
        G: Fn(&Metadata<'_>) -> bool + 'static,
    {
        MockCollector {
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
    /// `with_max_level_hint` is called on multiple collectors, the
    /// global filter will be the least restrictive of all collectors.
    /// To filter the events evaluated by a specific `MockCollector`,
    /// use [`with_filter`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{collector, expect};
    ///
    /// let (collector, handle) = collector::mock()
    ///     .with_max_level_hint(tracing::Level::INFO)
    ///     .event(expect::event().at_level(tracing::Level::INFO))
    ///     .only()
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
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
    /// use tracing_mock::{collector, expect};
    ///
    /// let (collector, handle) = collector::mock()
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
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
    /// use tracing_mock::{collector, expect};
    ///
    /// let (collector, handle) = collector::mock()
    ///     .event(expect::event())
    ///     .only()
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
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

    /// Consume the receiver and return an `impl` [`Collect`] which can
    /// be set as the default collector.
    ///
    /// This function is similar to [`run_with_handle`], but it doesn't
    /// return a [`MockHandle`]. This is useful if the desired
    /// assertions can be checked externally to the collector.
    ///
    /// # Examples
    ///
    /// The following test is used within the `tracing`
    /// codebase:
    ///
    /// ```
    /// use tracing_mock::collector;
    ///
    /// tracing::collect::with_default(collector::mock().run(), || {
    ///     let foo1 = tracing::span!(tracing::Level::TRACE, "foo");
    ///     let foo2 = foo1.clone();
    ///     // Two handles that point to the same span are equal.
    ///     assert_eq!(foo1, foo2);
    /// });
    /// ```
    ///
    /// [`Collect`]: tracing::Collect
    /// [`run_with_handle`]: fn@Self::run_with_handle
    pub fn run(self) -> impl Collect {
        let (collector, _) = self.run_with_handle();
        collector
    }

    /// Consume the receiver and return an `impl` [`Collect`] which can
    /// be set as the default collector and a [`MockHandle`] which can
    /// be used to validate the provided expectations.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{collector, expect};
    ///
    /// // collector and handle are returned from `run_with_handle()`
    /// let (collector, handle) = collector::mock()
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     tracing::info!("a");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`Collect`]: tracing::Collect
    pub fn run_with_handle(self) -> (impl Collect, MockHandle) {
        let expected = Arc::new(Mutex::new(self.expected));
        let handle = MockHandle(expected.clone(), self.name.clone());
        let collector = Running {
            spans: Mutex::new(HashMap::new()),
            expected,
            current: Mutex::new(Vec::new()),
            ids: AtomicUsize::new(1),
            filter: self.filter,
            max_level: self.max_level,
            name: self.name,
        };
        (collector, handle)
    }
}

impl<F> Collect for Running<F>
where
    F: Fn(&Metadata<'_>) -> bool + 'static,
{
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
                            "Expected scope for events is not supported with `MockCollector`."
                        )
                    }
                }
                let get_parent_name = || {
                    let stack = self.current.lock().unwrap();
                    let spans = self.spans.lock().unwrap();
                    event
                        .parent()
                        .and_then(|id| spans.get(id))
                        .or_else(|| stack.last().and_then(|id| spans.get(id)))
                        .map(|s| s.name.to_string())
                };
                expected.check(event, get_parent_name, &self.name);
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
                let get_parent_name = || {
                    let stack = self.current.lock().unwrap();
                    span.parent()
                        .and_then(|id| spans.get(id))
                        .or_else(|| stack.last().and_then(|id| spans.get(id)))
                        .map(|s| s.name.to_string())
                };
                expected.check(span, get_parent_name, &self.name);
            }
        }
        spans.insert(
            id.clone(),
            SpanState {
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
                    expected_span.check(span, &self.name);
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
                expected_span.check(span, &self.name);
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
        let name = self.spans.lock().unwrap().get_mut(id).map(|span| {
            let name = span.name;
            println!(
                "[{}] clone_span: {}; id={:?}; refs={:?};",
                self.name, name, id, span.refs
            );
            span.refs += 1;
            name
        });
        if name.is_none() {
            println!("[{}] clone_span: id={:?};", self.name, id);
        }
        let mut expected = self.expected.lock().unwrap();
        let was_expected = if let Some(Expect::CloneSpan(ref span)) = expected.front() {
            assert_eq!(
                name,
                span.name(),
                "[{}] expected to clone a span named {:?}",
                self.name,
                span.name()
            );
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

impl MockHandle {
    #[cfg(feature = "tracing-subscriber")]
    pub(crate) fn new(expected: Arc<Mutex<VecDeque<Expect>>>, name: String) -> Self {
        Self(expected, name)
    }

    /// Checks the expectations which were set on the
    /// [`MockCollector`].
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
    /// use tracing_mock::{collector, expect};
    ///
    /// let (collector, handle) = collector::mock()
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     tracing::info!("a");
    /// });
    ///
    /// // Check assertions set on the mock collector
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
