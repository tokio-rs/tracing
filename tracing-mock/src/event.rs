//! An [`ExpectedEvent`] defines an event to be matched by the mock
//! collector API in the [`collector`] module.
//!
//! The expected event should be created with [`expect::event`] and a
//! chain of method calls to describe the assertions we wish to make
//! about the event.
//!
//! # Examples
//!
//! ```
//! use tracing::collect::with_default;
//! use tracing_mock::{collector, expect};
//!
//! let event = expect::event()
//!     .at_level(tracing::Level::INFO)
//!     .with_fields(expect::field("field.name").with_value(&"field_value"));
//!
//! let (collector, handle) = collector::mock()
//!     .event(event)
//!     .run_with_handle();
//!
//! with_default(collector, || {
//!     tracing::info!(field.name = "field_value");
//! });
//!
//! handle.assert_finished();
//! ```
//!
//! [`collector`]: mod@crate::collector
//! [`expect::event`]: fn@crate::expect::event
#![allow(missing_docs)]
use super::{expect, field, metadata::ExpectedMetadata, span, Parent};

use std::fmt;

/// An expected event.
///
/// For a detailed description and examples see the documentation for
/// the methods and the [`event`] module.
///
/// [`event`]: mod@crate::event
#[derive(Default, Eq, PartialEq)]
pub struct ExpectedEvent {
    pub(super) fields: Option<field::ExpectedFields>,
    pub(super) parent: Option<Parent>,
    pub(super) in_spans: Vec<span::ExpectedSpan>,
    pub(super) metadata: ExpectedMetadata,
}

pub fn msg(message: impl fmt::Display) -> ExpectedEvent {
    expect::event().with_fields(field::msg(message))
}

impl ExpectedEvent {
    /// Sets the expected name to match an event.
    ///
    /// By default an event's name takes takes the form:
    /// `event <file>:<line>` where `<file>` and `<line>` refer to the
    /// location in the source code where the event was generated.
    ///
    /// To overwrite the name of an event, it has to be constructed
    /// directly instead of using one of the available macros.
    ///
    /// In general, there are not many use cases for expecting an
    /// event, as the value includes the file name and line number,
    /// which can make it quite a fragile check.
    pub fn named<I>(self, name: I) -> Self
    where
        I: Into<String>,
    {
        Self {
            metadata: ExpectedMetadata {
                name: Some(name.into()),
                ..self.metadata
            },
            ..self
        }
    }

    /// Adds fields to expect when matching an event.
    ///
    /// If an event is recorded with fields that do not match the provided
    /// [`ExpectedFields`], this expectation will fail.
    ///
    /// More information on the available validations is available on
    /// the [`ExpectedFields`] documentation.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing::collect::with_default;
    /// use tracing_mock::{collector, expect};
    ///
    /// let event = expect::event()
    ///     .with_fields(expect::field("field.name").with_value(&"field_value"));
    ///
    /// let (collector, handle) = collector::mock()
    ///     .event(event)
    ///     .run_with_handle();
    ///
    /// with_default(collector, || {
    ///     tracing::info!(field.name = "field_value");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`ExpectedFields`]: struct@crate::field::ExpectedFields
    pub fn with_fields<I>(self, fields: I) -> Self
    where
        I: Into<field::ExpectedFields>,
    {
        Self {
            fields: Some(fields.into()),
            ..self
        }
    }

    /// Sets the [`Level`](tracing::Level) to expect when matching an event.
    ///
    /// If an event is recorded at a different level, this expectation
    /// will fail.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing::collect::with_default;
    /// use tracing_mock::{collector, expect};
    ///
    /// let event = expect::event()
    ///     .at_level(tracing::Level::WARN);
    ///
    /// let (collector, handle) = collector::mock()
    ///     .event(event)
    ///     .run_with_handle();
    ///
    /// with_default(collector, || {
    ///     tracing::warn!("this message is bad news");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    pub fn at_level(self, level: tracing::Level) -> Self {
        Self {
            metadata: ExpectedMetadata {
                level: Some(level),
                ..self.metadata
            },
            ..self
        }
    }

    /// Sets the target to expect when matching events.
    ///
    /// If an event is recorded with a different target, this expectation will fail.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing::collect::with_default;
    /// use tracing_mock::{collector, expect};
    ///
    /// let event = expect::event()
    ///     .with_target("some_target");
    ///
    /// let (collector, handle) = collector::mock()
    ///     .event(event)
    ///     .run_with_handle();
    ///
    /// with_default(collector, || {
    ///     tracing::info!(target: "some_target", field = &"value");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    pub fn with_target<I>(self, target: I) -> Self
    where
        I: Into<String>,
    {
        Self {
            metadata: ExpectedMetadata {
                target: Some(target.into()),
                ..self.metadata
            },
            ..self
        }
    }

    /// Configures this `ExpectedEvent` to expect an explicit parent span
    /// when matching events or to be an explicit root.
    ///
    /// An _explicit_ parent span is one passed to the `span!` macro in the
    /// `parent:` field.
    ///
    /// If `Some("parent_name")` is passed to `with_explicit_parent` then
    /// the provided string is the name of the parent span to expect.
    ///
    /// To expect that an event is recorded with `parent: None`, `None`
    /// can be passed to `with_explicit_parent` instead.

    ///
    /// Use [`ExpectedEvent::without_explicit_parent`] to expect that an event
    /// is an explicit root (the span macro is invoked with `parent: None`).
    ///
    /// # Examples
    ///
    /// The explicit parent is matched by name.
    ///
    /// ```
    /// use tracing::collect::with_default;
    /// use tracing_mock::{collector, expect};
    ///
    /// let event = expect::event()
    ///     .with_explicit_parent(Some("parent_span"));
    ///
    /// let (collector, handle) = collector::mock()
    ///     .event(event)
    ///     .run_with_handle();
    ///
    /// with_default(collector, || {
    ///     let parent = tracing::info_span!("parent_span");
    ///     tracing::info!(parent: parent.id(), field = &"value");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// In the following example, we expect that the matched event is
    /// an explicit root.
    ///
    /// ```
    /// use tracing::collect::with_default;
    /// use tracing_mock::{collector, expect};
    ///
    /// let event = expect::event()
    ///     .with_explicit_parent(None);
    ///
    /// let (collector, handle) = collector::mock()
    ///     .event(event)
    ///     .run_with_handle();
    ///
    /// with_default(collector, || {
    ///     tracing::info!(parent: None, field = &"value");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// If an event is recorded without an explicit parent, or if the
    /// explicit parent has a different name, this expectation will
    /// fail. In the example below, the expectation fails because the
    /// event is contextually (and not explicitly) within the span
    /// `parent_span`.
    ///
    /// ```should_panic
    /// use tracing::collect::with_default;
    /// use tracing_mock::{collector, expect};
    ///
    /// let event = expect::event()
    ///     .with_explicit_parent(Some("parent_span"));
    ///
    /// let (collector, handle) = collector::mock()
    ///     .enter(expect::span())
    ///     .event(event)
    ///     .run_with_handle();
    ///
    /// with_default(collector, || {
    ///     let parent = tracing::info_span!("parent_span");
    ///     let _guard = parent.enter();
    ///     tracing::info!(field = &"value");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    pub fn with_explicit_parent(self, parent: Option<&str>) -> ExpectedEvent {
        let parent = match parent {
            Some(name) => Parent::Explicit(name.into()),
            None => Parent::ExplicitRoot,
        };
        Self {
            parent: Some(parent),
            ..self
        }
    }

    /// Configures this `ExpectedEvent` to match an event with a
    /// contextually-determined parent span.
    ///
    /// The provided string is the name of the parent span to expect.
    /// To expect that the event is a contextually-determined root, pass
    /// `None` instead.
    ///
    /// If an event is recorded which is not inside a span, has an explicitly
    /// overridden parent span, or with a differently-named span as its
    /// parent, this expectation will fail.
    ///
    /// To expect an event with an explicit parent span, use
    /// [`ExpectedEvent::with_explicit_parent`].
    ///
    /// # Examples
    ///
    /// The explicit parent is matched by name.
    ///
    /// ```
    /// use tracing::collect::with_default;
    /// use tracing_mock::{collector, expect};
    ///
    /// let event = expect::event()
    ///     .with_contextual_parent(Some("parent_span"));
    ///
    /// let (collector, handle) = collector::mock()
    ///     .enter(expect::span())
    ///     .event(event)
    ///     .run_with_handle();
    ///
    /// with_default(collector, || {
    ///     let parent = tracing::info_span!("parent_span");
    ///     let _guard = parent.enter();
    ///     tracing::info!(field = &"value");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// Matching an event recorded outside of a span.
    ///
    /// ```
    /// use tracing::collect::with_default;
    /// use tracing_mock::{collector, expect};
    ///
    /// let event = expect::event()
    ///     .with_contextual_parent(None);
    ///
    /// let (collector, handle) = collector::mock()
    ///     .event(event)
    ///     .run_with_handle();
    ///
    /// with_default(collector, || {
    ///     tracing::info!(field = &"value");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// This expectation will fail if the event is recorded with an
    /// explicit parent.
    ///
    /// If an event is recorded without an explicit parent, or if the
    /// explicit parent has a different name, this expectation will
    /// fail. In the example below, the expectation fails because the
    /// event is contextually (and not explicitly) within the span
    /// `parent_span`.
    ///
    /// ```should_panic
    /// use tracing::collect::with_default;
    /// use tracing_mock::{collector, expect};
    ///
    /// let event = expect::event()
    ///     .with_contextual_parent(Some("parent_span"));
    ///
    /// let (collector, handle) = collector::mock()
    ///     .enter(expect::span())
    ///     .event(event)
    ///     .run_with_handle();
    ///
    /// with_default(collector, || {
    ///     let parent = tracing::info_span!("parent_span");
    ///     tracing::info!(parent: parent.id(), field = &"value");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    pub fn with_contextual_parent(self, parent: Option<&str>) -> ExpectedEvent {
        let parent = match parent {
            Some(name) => Parent::Contextual(name.into()),
            None => Parent::ContextualRoot,
        };
        Self {
            parent: Some(parent),
            ..self
        }
    }

    /// Validates that the event is emitted within the scope of the
    /// provided `spans`.
    ///
    /// **Note**: This validation currently only works with a
    /// [`MockSubscriber`], it doesn't perform any validation when used
    /// with a [`MockCollector`].
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{collector, expect};
    /// use tracing_subscriber::{
    ///     filter::filter_fn, registry, subscribe::CollectExt, util::SubscriberInitExt, Subscribe,
    /// };
    ///
    /// let span1 = expect
    /// let event = expect::event()
    ///     .in_scope([expect::span("parent_span")]);
    ///
    /// let (subscriber, handle) = subscriber::mock()
    ///     .enter(expect::span())
    ///     .event(event)
    ///     .run_with_handle();
    ///
    /// let _collect = registry()
    ///     .with(subscriber.with_filter(filter_fn(move |_meta| true)))
    ///     .set_default();
    ///
    /// let parent = tracing::info_span!("parent_span");
    /// let _guard = parent.enter();
    /// tracing::info!(field = &"value");
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`MockSubscriber`]: struct@crate::subscriber::MockSubscriber
    /// [`MockCollector`]: struct@crate::collector::MockCollector
    #[cfg(feature = "tracing-subscriber")]
    pub fn in_scope(self, spans: impl IntoIterator<Item = span::ExpectedSpan>) -> Self {
        Self {
            in_spans: spans.into_iter().collect(),
            ..self
        }
    }

    /// Provides access to the expected scope (spans) for this expected
    /// event.
    #[cfg(feature = "tracing-subscriber")]
    pub(crate) fn scope_mut(&mut self) -> &mut [span::ExpectedSpan] {
        &mut self.in_spans[..]
    }

    pub(crate) fn check(
        &mut self,
        event: &tracing::Event<'_>,
        get_parent_name: impl FnOnce() -> Option<String>,
        collector_name: &str,
    ) {
        let meta = event.metadata();
        let name = meta.name();
        self.metadata
            .check(meta, format_args!("event \"{}\"", name), collector_name);
        assert!(
            meta.is_event(),
            "[{}] expected {}, but got {:?}",
            collector_name,
            self,
            event
        );
        if let Some(ref mut expected_fields) = self.fields {
            let mut checker = expected_fields.checker(name, collector_name);
            event.record(&mut checker);
            checker.finish();
        }

        if let Some(ref expected_parent) = self.parent {
            let actual_parent = get_parent_name();
            expected_parent.check_parent_name(
                actual_parent.as_deref(),
                event.parent().cloned(),
                event.metadata().name(),
                collector_name,
            )
        }
    }
}

impl fmt::Display for ExpectedEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "an event{}", self.metadata)
    }
}

impl fmt::Debug for ExpectedEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("MockEvent");

        if let Some(ref name) = self.metadata.name {
            s.field("name", name);
        }

        if let Some(ref target) = self.metadata.target {
            s.field("target", target);
        }

        if let Some(ref level) = self.metadata.level {
            s.field("level", &format_args!("{:?}", level));
        }

        if let Some(ref fields) = self.fields {
            s.field("fields", fields);
        }

        if let Some(ref parent) = self.parent {
            s.field("parent", &format_args!("{:?}", parent));
        }

        if !self.in_spans.is_empty() {
            s.field("in_spans", &self.in_spans);
        }

        s.finish()
    }
}
