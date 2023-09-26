//! An implementation of the [`Layer`] trait which validates that
//! the `tracing` data it recieves matches the expected output for a test.
//!
//!
//! The [`MockLayer`] is the central component in these tools. The
//! `MockLayer` has expectations set on it which are later
//! validated as the code under test is run.
//!
//! ```
//! use tracing_mock::{expect, field, layer};
//! use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
//!
//! let (layer, handle) = layer::mock()
//!     // Expect a single event with a specified message
//!     .event(expect::event().with_fields(field::msg("droids")))
//!     .run_with_handle();
//!
//! // Use `set_default` to apply the `MockSubscriber` until the end
//! // of the current scope (when the guard `_subscriber` is dropped).
//! let _subscriber =  tracing_subscriber::registry()
//!     .with(layer.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true)))
//!     .set_default();
//!
//! // These *are* the droids we are looking for
//! tracing::info!("droids");
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
//! use tracing_mock::{expect, field, layer};
//! use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
//!
//! let span = expect::span()
//!     .named("my_span");
//! let (layer, handle) = layer::mock()
//!     // Enter a matching span
//!     .enter(span.clone())
//!     // Record an event with message "collect parting message"
//!     .event(expect::event().with_fields(field::msg("say hello")))
//!     // Exit a matching span
//!     .exit(span)
//!     // Expect no further messages to be recorded
//!     .only()
//!     // Return the layer and handle
//!     .run_with_handle();
//!
//! // Use `set_default` to apply the `MockLayers` until the end
//! // of the current scope (when the guard `_subscriber` is dropped).
//! let _subscriber = tracing_subscriber::registry()
//!     .with(layer.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true)))
//!     .set_default();
//!
//! {
//!     let span = tracing::trace_span!(
//!         "my_span",
//!         greeting = "hello world",
//!     );
//!
//!     let _guard = span.enter();
//!     tracing::info!("say hello");
//! }
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
//! use tracing_mock::{expect, field, layer};
//! use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};  
//!
//! let span = expect::span()
//!     .named("my_span");
//! let (layer, handle) = layer::mock()
//!     // Enter a matching span
//!     .enter(span.clone())
//!     // Record an event with message "collect parting message"
//!     .event(expect::event().with_fields(field::msg("say hello")))
//!     // Exit a matching span
//!     .exit(span)
//!     // Expect no further messages to be recorded
//!     .only()
//!     // Return the subscriber and handle
//!     .run_with_handle();
//!
//! // Use `set_default` to apply the `MockSubscriber` until the end
//! // of the current scope (when the guard `_subscriber` is dropped).
//! let _subscriber =  tracing_subscriber::registry()
//!     .with(layer.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true)))
//!     .set_default();
//!
//! {
//!     let span = tracing::trace_span!(
//!         "my_span",
//!         greeting = "hello world",
//!     );
//!
//!     // Don't enter the span.
//!     // let _guard = span.enter();
//!     tracing::info!("say hello");
//! }
//!
//! // Use the handle to check the assertions. This line will panic if an
//! // assertion is not met.
//! handle.assert_finished();
//! ```
//!
//! [`Layer`]: trait@tracing_subscriber::layer::Layer
use crate::{
    event::ExpectedEvent,
    expect::Expect,
    span::{ExpectedSpan, NewSpan},
    subscriber::MockHandle,
};
use tracing_core::{
    span::{Attributes, Id, Record},
    Event, Subscriber,
};
use tracing_subscriber::{
    layer::{Context, Layer},
    registry::{LookupSpan, SpanRef},
};

use std::{
    collections::VecDeque,
    fmt,
    sync::{Arc, Mutex},
};

/// Create a [`MockLayerBuilder`] used to construct a
/// [`MockLayer`].
///
/// For additional information and examples, see the [`layer`]
/// module and [`MockLayerBuilder`] documentation.
///
/// # Examples
///
/// ```
/// use tracing_mock::{expect, field, layer};
/// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
///
/// let span = expect::span()
///     .named("my_span");
/// let (layer, handle) = layer::mock()
///     // Enter a matching span
///     .enter(span.clone())
///     // Record an event with message "collect parting message"
///     .event(expect::event().with_fields(field::msg("say hello")))
///     // Exit a matching span
///     .exit(span)
///     // Expect no further messages to be recorded
///     .only()
///     // Return the subscriber and handle
///     .run_with_handle();
///
/// // Use `set_default` to apply the `MockSubscriber` until the end
/// // of the current scope (when the guard `_subscriber` is dropped).
/// let _subscriber =  tracing_subscriber::registry()
///     .with(layer.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true)))
///     .set_default();
///
/// {
///     let span = tracing::trace_span!(
///         "my_span",
///         greeting = "hello world",
///     );
///
///     let _guard = span.enter();
///     tracing::info!("say hello");
/// }
///
/// // Use the handle to check the assertions. This line will panic if an
/// // assertion is not met.
/// handle.assert_finished();
/// ```
///
/// [`layer`]: mod@crate::layer
#[must_use]
pub fn mock() -> MockLayerBuilder {
    MockLayerBuilder {
        expected: Default::default(),
        name: std::thread::current()
            .name()
            .map(String::from)
            .unwrap_or_default(),
    }
}

/// Create a [`MockLayerBuilder`] with a name already set.
///
/// This constructor is equivalent to calling
/// [`MockLayerBuilder::named`] in the following way"
/// `layer::mock().named(name)`.
///
/// For additional information and examples, see the [`layer`]
/// module and [`MockLayerBuilder`] documentation.
///
/// # Examples
///
/// The example from [`named`] could be rewritten as:
///
/// ```should_panic
/// use tracing_mock::{expect, layer};
/// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
///
/// let (layer_1, handle_1) = layer::named("subscriber-1")
///     .event(expect::event())
///     .run_with_handle();
///
/// let (layer_2, handle_2) = layer::named("subscriber-2")
///     .event(expect::event())
///     .run_with_handle();
///
/// let _subscriber =  tracing_subscriber::registry()
///     .with(
///         layer_2.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true))
///     )
///     .set_default();
/// {
///     let _subscriber =  tracing_subscriber::registry()
///         .with(
///             layer_1
///                 .with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true))
///         )
///         .set_default();
///
///     tracing::info!("a");
/// }
///
/// handle_1.assert_finished();
/// handle_2.assert_finished();
/// ```
///
/// [`MockLayerBuilder::named`]: fn@crate::layer::MockLayerBuilder::named
/// [`layer`]: mod@crate::layer
#[must_use]
pub fn named(name: impl std::fmt::Display) -> MockLayerBuilder {
    mock().named(name)
}

/// A builder for constructing [`MockLayer`]s.
///
/// The methods on this builder set expectations which are then
/// validated by the constructed [`MockLayer`].
///
/// For a detailed description and examples see the documentation
/// for the methods and the [`layer`] module.
///
/// [`layer`]: mod@crate::layer

pub struct MockLayerBuilder {
    expected: VecDeque<Expect>,
    name: String,
}

/// A layer which validates the traces it receives.
///
/// A `MockLayer` is constructed with a
/// [`MockLayerBuilder`]. For a detailed description and examples,
/// see the documentation for that struct and for the [`layer`]
/// module.
///
/// [`layer`]: mod@crate::layer
pub struct MockLayer {
    expected: Arc<Mutex<VecDeque<Expect>>>,
    current: Mutex<Vec<Id>>,
    name: String,
}

impl MockLayerBuilder {
    /// Overrides the name printed by the mock layer's debugging output.
    ///
    /// The debugging output is displayed if the test panics, or if the test is
    /// run with `--nocapture`.
    ///
    /// By default, the mock layer's name is the  name of the test
    /// (*technically*, the name of the thread where it was created, which is
    /// the name of the test unless tests are run with `--test-threads=1`).
    /// When a test has only one mock layer, this is sufficient. However,
    /// some tests may include multiple layers, in order to test
    /// interactions between multiple layers. In that case, it can be
    /// helpful to give each layers a separate name to distinguish where the
    /// debugging output comes from.
    ///
    /// # Examples
    ///
    /// In the following example, we create two layers, both
    /// expecting to receive an event. As we only record a single
    /// event, the test will fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{layer, expect};
    /// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
    ///
    /// let (layer_1, handle_1) = layer::mock()
    ///     .named("layer-1")
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// let (layer_2, handle_2) = layer::mock()
    ///     .named("layer-2")
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// let _subscriber =  tracing_subscriber::registry()
    ///     .with(
    ///         layer_2.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true))
    ///     )
    ///     .set_default();
    /// {
    ///     let _subscriber =  tracing_subscriber::registry()
    ///         .with(
    ///             layer_1
    ///                 .with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true))
    ///         )
    ///         .set_default();
    ///
    ///     tracing::info!("a");
    /// }
    ///
    /// handle_1.assert_finished();
    /// handle_2.assert_finished();
    /// ```
    ///
    /// In the test output, we see that the layer which didn't
    /// received the event was the one named `layer-2`, which is
    /// correct as the layer named `layer-1` was the default
    /// when the event was recorded:
    ///
    /// ```text
    /// [main::layer-2] more notifications expected: [
    ///     Event(
    ///         MockEvent,
    ///     ),
    /// ]', tracing-mock/src/subscriber.rs:472:13
    /// ```
    pub fn named(mut self, name: impl fmt::Display) -> Self {
        use std::fmt::Write;
        if !self.name.is_empty() {
            write!(&mut self.name, "::{}", name).unwrap();
        } else {
            self.name = name.to_string();
        }
        self
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
    /// use tracing_mock::{expect, layer};
    /// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
    ///
    /// let (layer, handle) = layer::mock()
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// let _subscriber = tracing_subscriber::registry()
    ///     .with(layer.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true)))
    ///     .set_default();
    ///
    /// tracing::info!("event");
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// A span is entered before the event, causing the test to fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{expect, layer};
    /// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
    ///
    /// let (layer, handle) = layer::mock()
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// let _subscriber =  tracing_subscriber::registry()
    ///     .with(layer.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true)))
    ///     .set_default();
    ///
    /// let span = tracing::info_span!("span");
    /// let _guard = span.enter();
    /// tracing::info!("event");
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
    /// [`ExpectedSpan`] directly. [`NewSpan`] can be used to test
    /// span fields and the span parent.
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
    /// use tracing_mock::{expect, layer};
    /// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing")
    ///     .with_field(expect::field("testing").with_value(&"yes"));
    /// let (layer, handle) = layer::mock()
    ///     .new_span(span)
    ///     .run_with_handle();
    ///
    /// let _subscriber =  tracing_subscriber::registry()
    ///     .with(layer.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true)))
    ///     .set_default();
    ///
    /// _ = tracing::info_span!("the span we're testing", testing = "yes");
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// An event is recorded before the span is created, causing the
    /// test to fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{expect, layer};
    /// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing")
    ///     .with_field(expect::field("testing").with_value(&"yes"));
    /// let (layer, handle) = layer::mock()
    ///     .new_span(span)
    ///     .run_with_handle();
    ///
    /// let _subscriber =  tracing_subscriber::registry()
    ///     .with(layer.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true)))
    ///     .set_default();
    ///
    /// tracing::info!("an event");
    /// _ = tracing::info_span!("the span we're testing", testing = "yes");
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`ExpectedSpan`]: struct@crate::span::ExpectedSpan
    /// [`NewSpan`]: struct@crate::span::NewSpan
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
    /// [`exit`], since an entered span will typically be exited. If used
    /// together with [`only`], this is likely necessary, because the span
    /// will be dropped before the test completes (except in rare cases,
    /// such as if [`std::mem::forget`] is used).
    ///
    /// If the span that is entered doesn't match the [`ExpectedSpan`],
    /// or if something else (such as an event) is recorded first,
    /// then the expectation will fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{expect, layer};
    /// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing");
    /// let (layer, handle) = layer::mock()
    ///     .enter(span.clone())
    ///     .exit(span)
    ///     .only()
    ///     .run_with_handle();
    ///
    /// let _subscriber =  tracing_subscriber::registry()
    ///     .with(layer.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true)))
    ///     .set_default();
    ///
    /// {
    ///     let span = tracing::info_span!("the span we're testing");
    ///     let _entered = span.enter();
    /// }
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// An event is recorded before the span is entered, causing the
    /// test to fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{expect, layer};
    /// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing");
    /// let (layer, handle) = layer::mock()
    ///     .enter(span.clone())
    ///     .exit(span)
    ///     .only()
    ///     .run_with_handle();
    ///
    /// let _subscriber =  tracing_subscriber::registry()
    ///     .with(layer.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true)))
    ///     .set_default();
    ///
    /// {
    ///     tracing::info!("an event");
    ///     let span = tracing::info_span!("the span we're testing");
    ///     let _entered = span.enter();
    /// }
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

    /// Adds an expectation that exiting a span matching the
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
    /// **Note**: Ensure that the guard returned by [`Span::enter`]
    /// is dropped before calling [`MockHandle::assert_finished`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{expect, layer};
    /// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing");
    /// let (layer, handle) = layer::mock()
    ///     .enter(span.clone())
    ///     .exit(span)
    ///     .only()
    ///     .run_with_handle();
    ///
    /// let _subscriber =  tracing_subscriber::registry()
    ///     .with(layer.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true)))
    ///     .set_default();
    /// {
    ///     let span = tracing::info_span!("the span we're testing");
    ///     let _entered = span.enter();
    /// }
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// An event is recorded before the span is exited, causing the
    /// test to fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{expect, layer};
    /// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO)
    ///     .named("the span we're testing");
    /// let (layer, handle) = layer::mock()
    ///     .enter(span.clone())
    ///     .exit(span)
    ///     .only()
    ///     .run_with_handle();
    ///
    /// let _subscriber =  tracing_subscriber::registry()
    ///     .with(layer.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true)))
    ///     .set_default();
    ///
    /// {
    ///     let span = tracing::info_span!("the span we're testing");
    ///     let _entered = span.enter();
    ///     tracing::info!("an event");
    /// }
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`enter`]: fn@Self::enter
    /// [`MockHandle::assert_finished`]: fn@crate::subscriber::MockHandle::assert_finished
    /// [`Span::enter`]: fn@tracing::Span::enter
    pub fn exit(mut self, span: ExpectedSpan) -> Self {
        self.expected.push_back(Expect::Exit(span));
        self
    }

    /// Expects that no further traces are received.
    ///
    /// The call to `only` should appear immediately before the final
    /// call to [`run`] or [`run_with_handle`], as any expectations which
    /// are added after `only` will not be considered.
    ///
    /// # Examples
    ///
    /// Consider this simple test. It passes even though we only
    /// expect a single event, but receive three:
    ///
    /// ```
    /// use tracing_mock::{expect, layer};
    /// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
    ///
    /// let (layer, handle) = layer::mock()
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// let _subscriber = tracing_subscriber::registry()
    ///     .with(layer.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true)))
    ///     .set_default();
    ///
    /// tracing::info!("a");
    /// tracing::info!("b");
    /// tracing::info!("c");
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// After including `only`, the test will fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{expect, layer};
    /// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
    ///
    /// let (layer, handle) = layer::mock()
    ///     .event(expect::event())
    ///     .only()
    ///     .run_with_handle();
    ///
    /// let _subscriber =  tracing_subscriber::registry()
    ///     .with(layer.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true)))
    ///     .set_default();
    ///
    /// tracing::info!("a");
    /// tracing::info!("b");
    /// tracing::info!("c");
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`run`]: fn@Self::run
    /// [`run_with_handle`]: fn@Self::run_with_handle
    pub fn only(mut self) -> Self {
        self.expected.push_back(Expect::Nothing);
        self
    }

    /// Consume this builder and return a [`MockLayer`] which can
    /// be set as the default subscriber.
    ///
    /// This function is similar to [`run_with_handle`], but it doesn't
    /// return a [`MockHandle`]. This is useful if the desired
    /// assertions can be checked externally to the subscriber.
    ///
    /// # Examples
    ///
    /// The following test is used within the `tracing-subscriber`
    /// codebase:
    ///
    /// ```
    /// use tracing::Subscriber;
    /// use tracing_mock::layer;
    /// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
    ///
    /// let unfiltered = layer::named("unfiltered").run().boxed();
    /// let info = layer::named("info")
    ///     .run()
    ///     .with_filter(tracing_core::LevelFilter::INFO)
    ///     .boxed();
    /// let debug = layer::named("debug")
    ///     .run()
    ///     .with_filter(tracing_core::LevelFilter::DEBUG)
    ///     .boxed();
    ///
    /// let subscriber = tracing_subscriber::registry().with(vec![unfiltered, info, debug]);
    ///
    /// assert_eq!(subscriber.max_level_hint(), None);
    /// ```
    ///
    /// [`MockHandle`]: struct@crate::subscriber::MockHandle
    /// [`run_with_handle`]: fn@Self::run_with_handle
    pub fn run(self) -> MockLayer {
        MockLayer {
            expected: Arc::new(Mutex::new(self.expected)),
            name: self.name,
            current: Mutex::new(Vec::new()),
        }
    }

    /// Consume this builder and return a [`MockLayer`] which can
    /// be set as the default subscriber and a [`MockHandle`] which can
    /// be used to validate the provided expectations.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{expect, layer};
    /// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
    ///
    /// let (layer, handle) = layer::mock()
    ///     .event(expect::event())
    ///     .run_with_handle();
    ///
    /// let _subscriber =  tracing_subscriber::registry()
    ///     .with(layer.with_filter(tracing_subscriber::filter::filter_fn(move |_meta| true)))
    ///     .set_default();
    ///
    /// tracing::info!("event");
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`MockHandle`]: struct@crate::subscriber::MockHandle
    /// [`MockLayer`]: struct@crate::layer::MockLayer
    pub fn run_with_handle(self) -> (MockLayer, MockHandle) {
        let expected = Arc::new(Mutex::new(self.expected));
        let handle = MockHandle::new(expected.clone(), self.name.clone());
        let subscriber = MockLayer {
            expected,
            name: self.name,
            current: Mutex::new(Vec::new()),
        };
        (subscriber, handle)
    }
}

impl MockLayer {
    fn check_span_ref<'spans, S>(
        &self,
        expected: &ExpectedSpan,
        actual: &SpanRef<'spans, S>,
        what_happened: impl fmt::Display,
    ) where
        S: LookupSpan<'spans>,
    {
        if let Some(exp_name) = expected.name() {
            assert_eq!(
                actual.name(),
                exp_name,
                "\n[{}] expected {} a span named {:?}\n\
                 [{}] but it was named {:?} instead (span {} {:?})",
                self.name,
                what_happened,
                exp_name,
                self.name,
                actual.name(),
                actual.name(),
                actual.id()
            );
        }

        if let Some(exp_level) = expected.level() {
            let actual_level = actual.metadata().level();
            assert_eq!(
                actual_level,
                &exp_level,
                "\n[{}] expected {} a span at {:?}\n\
                 [{}] but it was at {:?} instead (span {} {:?})",
                self.name,
                what_happened,
                exp_level,
                self.name,
                actual_level,
                actual.name(),
                actual.id(),
            );
        }

        if let Some(exp_target) = expected.target() {
            let actual_target = actual.metadata().target();
            assert_eq!(
                actual_target,
                exp_target,
                "\n[{}] expected {} a span with target {:?}\n\
                 [{}] but it had the target {:?} instead (span {} {:?})",
                self.name,
                what_happened,
                exp_target,
                self.name,
                actual_target,
                actual.name(),
                actual.id(),
            );
        }
    }

    fn check_event_scope<C>(
        &self,
        current_scope: Option<tracing_subscriber::registry::Scope<'_, C>>,
        expected_scope: &mut [ExpectedSpan],
    ) where
        C: for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
    {
        let mut current_scope = current_scope.into_iter().flatten();
        let mut i = 0;
        for (expected, actual) in expected_scope.iter_mut().zip(&mut current_scope) {
            println!(
                "[{}] event_scope[{}] actual={} ({:?}); expected={}",
                self.name,
                i,
                actual.name(),
                actual.id(),
                expected
            );
            self.check_span_ref(
                expected,
                &actual,
                format_args!("the {}th span in the event's scope to be", i),
            );
            i += 1;
        }
        let remaining_expected = &expected_scope[i..];
        assert!(
            remaining_expected.is_empty(),
            "\n[{}] did not observe all expected spans in event scope!\n[{}] missing: {:#?}",
            self.name,
            self.name,
            remaining_expected,
        );
        assert!(
            current_scope.next().is_none(),
            "\n[{}] did not expect all spans in the actual event scope!",
            self.name,
        );
    }
}

impl<C> Layer<C> for MockLayer
where
    C: Subscriber + for<'a> LookupSpan<'a>,
{
    fn register_callsite(
        &self,
        metadata: &'static tracing::Metadata<'static>,
    ) -> tracing_core::Interest {
        println!("[{}] register_callsite {:#?}", self.name, metadata);
        tracing_core::Interest::always()
    }

    fn on_record(&self, _: &Id, _: &Record<'_>, _: Context<'_, C>) {
        unimplemented!(
            "so far, we don't have any tests that need an `on_record` \
            implementation.\nif you just wrote one that does, feel free to \
            implement it!"
        );
    }

    fn on_event(&self, event: &Event<'_>, cx: Context<'_, C>) {
        let name = event.metadata().name();
        println!(
            "[{}] event: {}; level: {}; target: {}",
            self.name,
            name,
            event.metadata().level(),
            event.metadata().target(),
        );
        match self.expected.lock().unwrap().pop_front() {
            None => {}
            Some(Expect::Event(mut expected)) => {
                let get_parent_name = || cx.event_span(event).map(|span| span.name().to_string());
                expected.check(event, get_parent_name, &self.name);

                if let Some(expected_scope) = expected.scope_mut() {
                    self.check_event_scope(cx.event_scope(event), expected_scope);
                }
            }
            Some(ex) => ex.bad(&self.name, format_args!("observed event {:#?}", event)),
        }
    }

    fn on_follows_from(&self, _span: &Id, _follows: &Id, _: Context<'_, C>) {
        unimplemented!(
            "so far, we don't have any tests that need an `on_follows_from` \
            implementation.\nif you just wrote one that does, feel free to \
            implement it!"
        );
    }

    fn on_new_span(&self, span: &Attributes<'_>, id: &Id, cx: Context<'_, C>) {
        let meta = span.metadata();
        println!(
            "[{}] new_span: name={:?}; target={:?}; id={:?};",
            self.name,
            meta.name(),
            meta.target(),
            id
        );
        let mut expected = self.expected.lock().unwrap();
        let was_expected = matches!(expected.front(), Some(Expect::NewSpan(_)));
        if was_expected {
            if let Expect::NewSpan(mut expected) = expected.pop_front().unwrap() {
                let get_parent_name = || {
                    span.parent()
                        .and_then(|id| cx.span(id))
                        .or_else(|| cx.lookup_current())
                        .map(|span| span.name().to_string())
                };
                expected.check(span, get_parent_name, &self.name);
            }
        }
    }

    fn on_enter(&self, id: &Id, cx: Context<'_, C>) {
        let span = cx
            .span(id)
            .unwrap_or_else(|| panic!("[{}] no span for ID {:?}", self.name, id));
        println!("[{}] enter: {}; id={:?};", self.name, span.name(), id);
        match self.expected.lock().unwrap().pop_front() {
            None => {}
            Some(Expect::Enter(ref expected_span)) => {
                self.check_span_ref(expected_span, &span, "to enter");
            }
            Some(ex) => ex.bad(&self.name, format_args!("entered span {:?}", span.name())),
        }
        self.current.lock().unwrap().push(id.clone());
    }

    fn on_exit(&self, id: &Id, cx: Context<'_, C>) {
        if std::thread::panicking() {
            // `exit()` can be called in `drop` impls, so we must guard against
            // double panics.
            println!("[{}] exit {:?} while panicking", self.name, id);
            return;
        }
        let span = cx
            .span(id)
            .unwrap_or_else(|| panic!("[{}] no span for ID {:?}", self.name, id));
        println!("[{}] exit: {}; id={:?};", self.name, span.name(), id);
        match self.expected.lock().unwrap().pop_front() {
            None => {}
            Some(Expect::Exit(ref expected_span)) => {
                self.check_span_ref(expected_span, &span, "to exit");
                let curr = self.current.lock().unwrap().pop();
                assert_eq!(
                    Some(id),
                    curr.as_ref(),
                    "[{}] exited span {:?}, but the current span was {:?}",
                    self.name,
                    span.name(),
                    curr.as_ref().and_then(|id| cx.span(id)).map(|s| s.name())
                );
            }
            Some(ex) => ex.bad(&self.name, format_args!("exited span {:?}", span.name())),
        };
    }

    fn on_close(&self, id: Id, cx: Context<'_, C>) {
        if std::thread::panicking() {
            // `try_close` can be called in `drop` impls, so we must guard against
            // double panics.
            println!("[{}] close {:?} while panicking", self.name, id);
            return;
        }
        let span = cx.span(&id);
        let name = span.as_ref().map(|span| {
            println!("[{}] close_span: {}; id={:?};", self.name, span.name(), id,);
            span.name()
        });
        if name.is_none() {
            println!("[{}] drop_span: id={:?}", self.name, id);
        }
        if let Ok(mut expected) = self.expected.try_lock() {
            let was_expected = match expected.front() {
                Some(Expect::DropSpan(ref expected_span)) => {
                    // Don't assert if this function was called while panicking,
                    // as failing the assertion can cause a double panic.
                    if !::std::thread::panicking() {
                        if let Some(ref span) = span {
                            self.check_span_ref(expected_span, span, "to close");
                        }
                    }
                    true
                }
                Some(Expect::Event(_)) => {
                    if !::std::thread::panicking() {
                        panic!(
                            "[{}] expected an event, but dropped span {} (id={:?}) instead",
                            self.name,
                            name.unwrap_or("<unknown name>"),
                            id
                        );
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

    fn on_id_change(&self, _old: &Id, _new: &Id, _ctx: Context<'_, C>) {
        panic!("well-behaved subscribers should never do this to us, lol");
    }
}

impl fmt::Debug for MockLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("ExpectSubscriber");
        s.field("name", &self.name);

        if let Ok(expected) = self.expected.try_lock() {
            s.field("expected", &expected);
        } else {
            s.field("expected", &format_args!("<locked>"));
        }

        if let Ok(current) = self.current.try_lock() {
            s.field("current", &format_args!("{:?}", &current));
        } else {
            s.field("current", &format_args!("<locked>"));
        }

        s.finish()
    }
}
