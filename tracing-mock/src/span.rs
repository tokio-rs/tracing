//! Define expectations to match and validate spans.
//!
//! The [`ExpectedSpan`] and [`NewSpan`] structs define expectations
//! for spans to be matched by the mock collector API in the
//! [`collector`] module.
//!
//! Expected spans should be created with [`expect::span`] and a
//! chain of method calls describing the assertions made about the
//! span. Expectations about the lifecycle of the span can be set on the [`MockCollector`].
//!
//! # Examples
//!
//! ```
//! use tracing_mock::{collector, expect};
//!
//! let span = expect::span()
//!     .named("interesting_span")
//!     .at_level(tracing::Level::INFO);
//!
//! let (collector, handle) = collector::mock()
//!     .enter(span.clone())
//!     .exit(span)
//!     .run_with_handle();
//!
//! tracing::collect::with_default(collector, || {
//!    let span = tracing::info_span!("interesting_span");
//!     let _guard = span.enter();
//! });
//!
//! handle.assert_finished();
//! ```
//!
//! The following example asserts the name, level, parent, and fields of the span:
//!
//! ```
//! use tracing_mock::{collector, expect};
//!
//! let span = expect::span()
//!     .named("interesting_span")
//!     .at_level(tracing::Level::INFO);
//! let new_span = span
//!     .clone()
//!     .with_fields(expect::field("field.name").with_value(&"field_value"))
//!     .with_explicit_parent(Some("parent_span"));
//!
//! let (collector, handle) = collector::mock()
//!     .new_span(expect::span().named("parent_span"))
//!     .new_span(new_span)
//!     .enter(span.clone())
//!     .exit(span)
//!     .run_with_handle();
//!
//! tracing::collect::with_default(collector, || {
//!     let parent = tracing::info_span!("parent_span");
//!
//!     let span = tracing::info_span!(
//!         parent: parent.id(),
//!         "interesting_span",
//!         field.name = "field_value",
//!     );
//!     let _guard = span.enter();
//! });
//!
//! handle.assert_finished();
//! ```
//!
//! All expectations must be met for the test to pass. For example,
//! the following test will fail due to a mismatch in the spans' names:
//!
//! ```should_panic
//! use tracing_mock::{collector, expect};
//!
//! let span = expect::span()
//!     .named("interesting_span")
//!     .at_level(tracing::Level::INFO);
//!
//! let (collector, handle) = collector::mock()
//!     .enter(span.clone())
//!     .exit(span)
//!     .run_with_handle();
//!
//! tracing::collect::with_default(collector, || {
//!    let span = tracing::info_span!("another_span");
//!    let _guard = span.enter();
//! });
//!
//! handle.assert_finished();
//! ```
//!
//! [`MockCollector`]: struct@crate::collector::MockCollector
//! [`collector`]: mod@crate::collector
//! [`expect::span`]: fn@crate::expect::span
#![allow(missing_docs)]
use crate::{
    collector::SpanState, expect, field::ExpectedFields, metadata::ExpectedMetadata, Parent,
};
use std::fmt;

/// A mock span.
///
/// This is intended for use with the mock collector API in the
/// [`collector`] module.
///
/// [`collector`]: mod@crate::collector
#[derive(Clone, Default, Eq, PartialEq)]
pub struct ExpectedSpan {
    pub(crate) metadata: ExpectedMetadata,
}

/// A mock new span.
///
/// **Note**: This struct contains expectations that can only be asserted
/// on when expecting a new span via [`MockCollector::new_span`]. They
/// cannot be validated on [`MockCollector::enter`],
/// [`MockCollector::exit`], or any other method on [`MockCollector`]
/// that takes an `ExpectedSpan`.
///
/// For more details on how to use this struct, see the documentation
/// on the [`collector`] module.
///
/// [`collector`]: mod@crate::collector
/// [`MockCollector`]: struct@crate::collector::MockCollector
/// [`MockCollector::enter`]: fn@crate::collector::MockCollector::enter
/// [`MockCollector::exit`]: fn@crate::collector::MockCollector::exit
/// [`MockCollector::new_span`]: fn@crate::collector::MockCollector::new_span
#[derive(Default, Eq, PartialEq)]
pub struct NewSpan {
    pub(crate) span: ExpectedSpan,
    pub(crate) fields: ExpectedFields,
    pub(crate) parent: Option<Parent>,
}

pub fn named<I>(name: I) -> ExpectedSpan
where
    I: Into<String>,
{
    expect::span().named(name)
}

impl ExpectedSpan {
    /// Sets a name to expect when matching a span.
    ///
    /// If an event is recorded with a name that differs from the one provided to this method, the expectation will fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span().named("span name");
    ///
    /// let (collector, handle) = collector::mock()
    ///     .enter(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     let span = tracing::info_span!("span name");
    ///     let _guard = span.enter();
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// When the span name is different, the assertion will fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span().named("span name");
    ///
    /// let (collector, handle) = collector::mock()
    ///     .enter(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     let span = tracing::info_span!("a different span name");
    ///     let _guard = span.enter();
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    pub fn named<I>(self, name: I) -> Self
    where
        I: Into<String>,
    {
        Self {
            metadata: ExpectedMetadata {
                name: Some(name.into()),
                ..self.metadata
            },
        }
    }

    /// Sets the [`Level`](tracing::Level) to expect when matching a span.
    ///
    /// If an span is record with a level that differs from the one provided to this method, the expectation will fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO);
    ///
    /// let (collector, handle) = collector::mock()
    ///     .enter(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     let span = tracing::info_span!("span");
    ///     let _guard = span.enter();
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// Expecting a span at `INFO` level will fail if the event is
    /// recorded at any other level:
    ///
    /// ```should_panic
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .at_level(tracing::Level::INFO);
    ///
    /// let (collector, handle) = collector::mock()
    ///     .enter(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     let span = tracing::warn_span!("a serious span");
    ///     let _guard = span.enter();
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
        }
    }

    /// Sets the target to expect when matching a span.
    ///
    /// If an event is recorded with a target that doesn't match the
    /// provided target, this expectation will fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .with_target("some_target");
    ///
    /// let (collector, handle) = collector::mock()
    ///     .enter(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     let span = tracing::info_span!(target: "some_target", "span");
    ///     let _guard = span.enter();
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// The test will fail if the target is different:
    ///
    /// ```should_panic
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .with_target("some_target");
    ///
    /// let (collector, handle) = collector::mock()
    ///     .enter(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     let span = tracing::info_span!(target: "a_different_target", "span");
    ///     let _guard = span.enter();
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
        }
    }

    /// Configures this `ExpectedSpan` to expect an explicit parent
    /// span or to be an explicit root.
    ///
    /// **Note**: This method returns a [`NewSpan`] and as such, this
    /// expectation can only be validated when expecting a new span via
    /// [`MockCollector::new_span`]. It cannot be validated on
    /// [`MockCollector::enter`], [`MockCollector::exit`], or any other
    /// method on [`MockCollector`] that takes an `ExpectedSpan`.
    ///
    /// An _explicit_ parent span is one passed to the `span!` macro in the
    /// `parent:` field.
    ///
    /// If `Some("parent_name")` is passed to `with_explicit_parent` then,
    /// the provided string is the name of the parent span to expect.
    ///
    /// To expect that a span is recorded with no parent, `None`
    /// can be passed to `with_explicit_parent` instead.
    ///
    /// If a span is recorded without an explicit parent, or if the
    /// explicit parent has a different name, this expectation will
    /// fail.
    ///
    /// # Examples
    ///
    /// The explicit parent is matched by name:
    ///
    /// ```
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .with_explicit_parent(Some("parent_span"));
    ///
    /// let (collector, handle) = collector::mock()
    ///     .new_span(expect::span().named("parent_span"))
    ///     .new_span(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     let parent = tracing::info_span!("parent_span");
    ///     tracing::info_span!(parent: parent.id(), "span");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// In the following example, the expected span is an explicit root:
    ///
    /// ```
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .with_explicit_parent(None);
    ///
    /// let (collector, handle) = collector::mock()
    ///     .new_span(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     tracing::info_span!(parent: None, "span");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// In the example below, the expectation fails because the
    /// span is *contextually*—as opposed to explicitly—within the span
    /// `parent_span`:
    ///
    /// ```should_panic
    /// use tracing_mock::{collector, expect};
    ///
    /// let parent_span = expect::span().named("parent_span");
    /// let span = expect::span()
    ///     .with_explicit_parent(Some("parent_span"));
    ///
    /// let (collector, handle) = collector::mock()
    ///     .new_span(parent_span.clone())
    ///     .enter(parent_span)
    ///     .new_span(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     let parent = tracing::info_span!("parent_span");
    ///     let _guard = parent.enter();
    ///     tracing::info_span!("span");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`MockCollector`]: struct@crate::collector::MockCollector
    /// [`MockCollector::enter`]: fn@crate::collector::MockCollector::enter
    /// [`MockCollector::exit`]: fn@crate::collector::MockCollector::exit
    /// [`MockCollector::new_span`]: fn@crate::collector::MockCollector::new_span
    pub fn with_explicit_parent(self, parent: Option<&str>) -> NewSpan {
        let parent = match parent {
            Some(name) => Parent::Explicit(name.into()),
            None => Parent::ExplicitRoot,
        };
        NewSpan {
            parent: Some(parent),
            span: self,
            ..Default::default()
        }
    }

    /// Configures this `ExpectedSpan` to expect a
    /// contextually-determined parent span, or be a contextual
    /// root.
    ///
    /// **Note**: This method returns a [`NewSpan`] and as such, this
    /// expectation can only be validated when expecting a new span via
    /// [`MockCollector::new_span`]. It cannot be validated on
    /// [`MockCollector::enter`], [`MockCollector::exit`], or any other
    /// method on [`MockCollector`] that takes an `ExpectedSpan`.
    ///
    /// The provided string is the name of the parent span to expect.
    /// To expect that the event is a contextually-determined root, pass
    /// `None` instead.
    ///
    /// To expect a span with an explicit parent span, use
    /// [`ExpectedSpan::with_explicit_parent`].
    ///
    /// If a span is recorded which is not inside a span, has an explicitly
    /// overridden parent span, or has a differently-named span as its
    /// parent, this expectation will fail.
    ///
    /// # Examples
    ///
    /// The contextual parent is matched by name:
    ///
    /// ```
    /// use tracing_mock::{collector, expect};
    ///
    /// let parent_span = expect::span().named("parent_span");
    /// let span = expect::span()
    ///     .with_contextual_parent(Some("parent_span"));
    ///
    /// let (collector, handle) = collector::mock()
    ///     .new_span(parent_span.clone())
    ///     .enter(parent_span)
    ///     .new_span(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     let parent = tracing::info_span!("parent_span");
    ///     let _guard = parent.enter();
    ///     tracing::info_span!("span");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// In the following example, we expect that the matched span is
    /// a contextually-determined root:
    ///
    /// ```
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .with_contextual_parent(None);
    ///
    /// let (collector, handle) = collector::mock()
    ///     .new_span(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     tracing::info_span!("span");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// In the example below, the expectation fails because the
    /// span is recorded with an explicit parent:
    ///
    /// ```should_panic
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .with_contextual_parent(Some("parent_span"));
    ///
    /// let (collector, handle) = collector::mock()
    ///     .new_span(expect::span().named("parent_span"))
    ///     .new_span(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     let parent = tracing::info_span!("parent_span");
    ///     tracing::info_span!(parent: parent.id(), "span");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`MockCollector`]: struct@crate::collector::MockCollector
    /// [`MockCollector::enter`]: fn@crate::collector::MockCollector::enter
    /// [`MockCollector::exit`]: fn@crate::collector::MockCollector::exit
    /// [`MockCollector::new_span`]: fn@crate::collector::MockCollector::new_span
    pub fn with_contextual_parent(self, parent: Option<&str>) -> NewSpan {
        let parent = match parent {
            Some(name) => Parent::Contextual(name.into()),
            None => Parent::ContextualRoot,
        };
        NewSpan {
            parent: Some(parent),
            span: self,
            ..Default::default()
        }
    }

    /// Adds fields to expect when matching a span.
    ///
    /// **Note**: This method returns a [`NewSpan`] and as such, this
    /// expectation can only be validated when expecting a new span via
    /// [`MockCollector::new_span`]. It cannot be validated on
    /// [`MockCollector::enter`], [`MockCollector::exit`], or any other
    /// method on [`MockCollector`] that takes an `ExpectedSpan`.
    ///
    /// If a span is recorded with fields that do not match the provided
    /// [`ExpectedFields`], this expectation will fail.
    ///
    /// If the provided field is not present on the recorded span or
    /// if the value for that field diffs, then the expectation
    /// will fail.
    ///
    /// More information on the available validations is available in
    /// the [`ExpectedFields`] documentation.
    ///
    /// # Examples
    ///
    /// ```
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .with_fields(expect::field("field.name").with_value(&"field_value"));
    ///
    /// let (collector, handle) = collector::mock()
    ///     .new_span(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     tracing::info_span!("span", field.name = "field_value");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// A different field value will cause the expectation to fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .with_fields(expect::field("field.name").with_value(&"field_value"));
    ///
    /// let (collector, handle) = collector::mock()
    ///     .new_span(span)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     tracing::info_span!("span", field.name = "different_field_value");
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`ExpectedFields`]: struct@crate::field::ExpectedFields
    /// [`MockCollector`]: struct@crate::collector::MockCollector
    /// [`MockCollector::enter`]: fn@crate::collector::MockCollector::enter
    /// [`MockCollector::exit`]: fn@crate::collector::MockCollector::exit
    /// [`MockCollector::new_span`]: fn@crate::collector::MockCollector::new_span
    pub fn with_fields<I>(self, fields: I) -> NewSpan
    where
        I: Into<ExpectedFields>,
    {
        NewSpan {
            span: self,
            fields: fields.into(),
            ..Default::default()
        }
    }

    pub(crate) fn name(&self) -> Option<&str> {
        self.metadata.name.as_ref().map(String::as_ref)
    }

    pub(crate) fn level(&self) -> Option<tracing::Level> {
        self.metadata.level
    }

    pub(crate) fn target(&self) -> Option<&str> {
        self.metadata.target.as_deref()
    }

    pub(crate) fn check(&self, actual: &SpanState, collector_name: &str) {
        let meta = actual.metadata();
        let name = meta.name();
        self.metadata
            .check(meta, format_args!("span `{}`", name), collector_name);
    }
}

impl fmt::Debug for ExpectedSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("MockSpan");

        if let Some(name) = self.name() {
            s.field("name", &name);
        }

        if let Some(level) = self.level() {
            s.field("level", &format_args!("{:?}", level));
        }

        if let Some(target) = self.target() {
            s.field("target", &target);
        }

        s.finish()
    }
}

impl fmt::Display for ExpectedSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.metadata.name.is_some() {
            write!(f, "a span{}", self.metadata)
        } else {
            write!(f, "any span{}", self.metadata)
        }
    }
}

impl From<ExpectedSpan> for NewSpan {
    fn from(span: ExpectedSpan) -> Self {
        Self {
            span,
            ..Default::default()
        }
    }
}

impl NewSpan {
    /// Configures this `ExpectedSpan` to expect an explicit parent
    /// span or to be an explicit root.
    ///
    /// For more information and examples, see the documentation on
    /// [`ExpectedSpan::with_explicit_parent`].
    ///
    /// [`ExpectedSpan::with_explicit_parent`]: fn@crate::span::ExpectedSpan::with_explicit_parent
    pub fn with_explicit_parent(self, parent: Option<&str>) -> NewSpan {
        let parent = match parent {
            Some(name) => Parent::Explicit(name.into()),
            None => Parent::ExplicitRoot,
        };
        NewSpan {
            parent: Some(parent),
            ..self
        }
    }

    /// Configures this `NewSpan` to expect a
    /// contextually-determined parent span, or to be a contextual
    /// root.
    ///
    /// For more information and examples, see the documentation on
    /// [`ExpectedSpan::with_contextual_parent`].
    ///
    /// [`ExpectedSpan::with_contextual_parent`]: fn@crate::span::ExpectedSpan::with_contextual_parent
    pub fn with_contextual_parent(self, parent: Option<&str>) -> NewSpan {
        let parent = match parent {
            Some(name) => Parent::Contextual(name.into()),
            None => Parent::ContextualRoot,
        };
        NewSpan {
            parent: Some(parent),
            ..self
        }
    }

    /// Adds fields to expect when matching a span.
    ///
    /// For more information and examples, see the documentation on
    /// [`ExpectedSpan::with_fields`].
    ///
    /// [`ExpectedSpan::with_fields`]: fn@crate::span::ExpectedSpan::with_fields
    pub fn with_fields<I>(self, fields: I) -> NewSpan
    where
        I: Into<ExpectedFields>,
    {
        NewSpan {
            fields: fields.into(),
            ..self
        }
    }

    pub(crate) fn check(
        &mut self,
        span: &tracing_core::span::Attributes<'_>,
        get_parent_name: impl FnOnce() -> Option<String>,
        collector_name: &str,
    ) {
        let meta = span.metadata();
        let name = meta.name();
        self.span
            .metadata
            .check(meta, format_args!("span `{}`", name), collector_name);
        let mut checker = self.fields.checker(name, collector_name);
        span.record(&mut checker);
        checker.finish();

        if let Some(expected_parent) = self.parent.as_ref() {
            let actual_parent = get_parent_name();
            expected_parent.check_parent_name(
                actual_parent.as_deref(),
                span.parent().cloned(),
                format_args!("span `{}`", name),
                collector_name,
            )
        }
    }
}

impl fmt::Display for NewSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a new span{}", self.span.metadata)?;
        if !self.fields.is_empty() {
            write!(f, " with {}", self.fields)?;
        }
        Ok(())
    }
}

impl fmt::Debug for NewSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("NewSpan");

        if let Some(name) = self.span.name() {
            s.field("name", &name);
        }

        if let Some(level) = self.span.level() {
            s.field("level", &format_args!("{:?}", level));
        }

        if let Some(target) = self.span.target() {
            s.field("target", &target);
        }

        if let Some(ref parent) = self.parent {
            s.field("parent", &format_args!("{:?}", parent));
        }

        if !self.fields.is_empty() {
            s.field("fields", &self.fields);
        }

        s.finish()
    }
}
