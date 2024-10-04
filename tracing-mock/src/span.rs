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
//!     .enter(&span)
//!     .exit(&span)
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
//! Instead of passing an `ExpectedSpan`, the collector methods will also accept
//! anything that implements `Into<String>` which is shorthand for
//! `expect::span().named(name)`.
//!
//! ```
//! use tracing_mock::collector;
//!
//! let (collector, handle) = collector::mock()
//!     .enter("interesting_span")
//!     .run_with_handle();
//!
//! tracing::collect::with_default(collector, || {
//!    let span = tracing::info_span!("interesting_span");
//!     let _guard = span.enter();
//! });
//!
//! handle.assert_finished();
//! ```
//
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
//!     .with_ancestry(expect::has_explicit_parent("parent_span"));
//!
//! let (collector, handle) = collector::mock()
//!     .new_span("parent_span")
//!     .new_span(new_span)
//!     .enter(&span)
//!     .exit(&span)
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
//!     .enter(&span)
//!     .exit(&span)
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
    ancestry::Ancestry, collector::SpanState, expect, field::ExpectedFields,
    metadata::ExpectedMetadata,
};
use std::{
    error, fmt,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

/// A mock span.
///
/// This is intended for use with the mock collector API in the
/// [`collector`] module.
///
/// [`collector`]: mod@crate::collector
#[derive(Clone, Default, Eq, PartialEq)]
pub struct ExpectedSpan {
    pub(crate) id: Option<ExpectedId>,
    pub(crate) metadata: ExpectedMetadata,
}

impl<I> From<I> for ExpectedSpan
where
    I: Into<String>,
{
    fn from(name: I) -> Self {
        ExpectedSpan::default().named(name)
    }
}

impl From<&ExpectedId> for ExpectedSpan {
    fn from(id: &ExpectedId) -> Self {
        ExpectedSpan::default().with_id(id.clone())
    }
}

impl From<&ExpectedSpan> for ExpectedSpan {
    fn from(span: &ExpectedSpan) -> Self {
        span.clone()
    }
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
    pub(crate) ancestry: Option<Ancestry>,
}

pub fn named<I>(name: I) -> ExpectedSpan
where
    I: Into<String>,
{
    expect::span().named(name)
}

/// A mock span ID.
///
/// This ID makes it possible to link together calls to different
/// [`MockCollector`] span methods that take an [`ExpectedSpan`] in
/// addition to those that take a [`NewSpan`].
///
/// Use [`expect::id`] to construct a new, unset `ExpectedId`.
///
/// For more details on how to use this struct, see the documentation
/// on [`ExpectedSpan::with_id`].
///
/// [`expect::id`]: fn@crate::expect::id
/// [`MockCollector`]: struct@crate::collector::MockCollector
#[derive(Clone, Default)]
pub struct ExpectedId {
    inner: Arc<AtomicU64>,
}

impl ExpectedSpan {
    /// Sets a name to expect when matching a span.
    ///
    /// If an event is recorded with a name that differs from the one provided to this method, the
    /// expectation will fail.
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
    /// If only the name of the span needs to be validated, then
    /// instead of using the `named` method, a string can be passed
    /// to the [`MockCollector`] functions directly.
    ///
    /// ```
    /// use tracing_mock::collector;
    ///
    /// let (collector, handle) = collector::mock()
    ///     .enter("span name")
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
            ..self
        }
    }

    /// Sets the `ID` to expect when matching a span.
    ///
    /// The [`ExpectedId`] can be used to differentiate spans that are
    /// otherwise identical. An [`ExpectedId`] needs to be attached to
    /// an `ExpectedSpan` or [`NewSpan`] which is passed to
    /// [`MockCollector::new_span`]. The same [`ExpectedId`] can then
    /// be used to match the exact same span when passed to
    /// [`MockCollector::enter`], [`MockCollector::exit`], and
    /// [`MockCollector::drop_span`].
    ///
    /// This is especially useful when `tracing-mock` is being used to
    /// test the traces being generated within your own crate, in which
    /// case you may need to distinguish between spans which have
    /// identical metadata but different field values, which can
    /// otherwise only be checked in [`MockCollector::new_span`].
    ///
    /// # Examples
    ///
    /// Here we expect that the span that is created first is entered
    /// second:
    ///
    /// ```
    /// use tracing_mock::{collector, expect};
    /// let id1 = expect::id();
    /// let span1 = expect::span().named("span").with_id(id1.clone());
    /// let id2 = expect::id();
    /// let span2 = expect::span().named("span").with_id(id2.clone());
    ///
    /// let (collector, handle) = collector::mock()
    ///     .new_span(&span1)
    ///     .new_span(&span2)
    ///     .enter(&span2)
    ///     .enter(&span1)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     fn create_span() -> tracing::Span {
    ///         tracing::info_span!("span")
    ///     }
    ///
    ///     let span1 = create_span();
    ///     let span2 = create_span();
    ///
    ///     let _guard2 = span2.enter();
    ///     let _guard1 = span1.enter();
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// Since `ExpectedId` implements `Into<ExpectedSpan>`, in cases where
    /// only checking on Id is desired, a shorthand version of the previous
    /// example can be used.
    ///
    /// ```
    /// use tracing_mock::{collector, expect};
    /// let id1 = expect::id();
    /// let id2 = expect::id();
    ///
    /// let (collector, handle) = collector::mock()
    ///     .new_span(&id1)
    ///     .new_span(&id2)
    ///     .enter(&id2)
    ///     .enter(&id1)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     fn create_span() -> tracing::Span {
    ///         tracing::info_span!("span")
    ///     }
    ///
    ///     let span1 = create_span();
    ///     let span2 = create_span();
    ///
    ///     let _guard2 = span2.enter();
    ///     let _guard1 = span1.enter();
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// If the order that the spans are entered changes, the test will
    /// fail:
    ///
    /// ```should_panic
    /// use tracing_mock::{collector, expect};
    /// let id1 = expect::id();
    /// let span1 = expect::span().named("span").with_id(id1.clone());
    /// let id2 = expect::id();
    /// let span2 = expect::span().named("span").with_id(id2.clone());
    ///
    /// let (collector, handle) = collector::mock()
    ///     .new_span(&span1)
    ///     .new_span(&span2)
    ///     .enter(&span2)
    ///     .enter(&span1)
    ///     .run_with_handle();
    ///
    /// tracing::collect::with_default(collector, || {
    ///     fn create_span() -> tracing::Span {
    ///         tracing::info_span!("span")
    ///     }
    ///
    ///     let span1 = create_span();
    ///     let span2 = create_span();
    ///
    ///     let _guard1 = span1.enter();
    ///     let _guard2 = span2.enter();
    /// });
    ///
    /// handle.assert_finished();
    /// ```
    ///
    /// [`MockCollector::new_span`]: fn@crate::collector::MockCollector::new_span
    /// [`MockCollector::enter`]: fn@crate::collector::MockCollector::enter
    /// [`MockCollector::exit`]: fn@crate::collector::MockCollector::exit
    /// [`MockCollector::drop_span`]: fn@crate::collector::MockCollector::drop_span
    pub fn with_id(self, id: ExpectedId) -> Self {
        Self {
            id: Some(id),
            ..self
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
            ..self
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
            ..self
        }
    }

    /// Configures this `ExpectedSpan` to expect the specified [`Ancestry`]. A
    /// span's ancestry indicates whether it has a parent or is a root span
    /// and whether the parent is explitly or contextually assigned.
    ///
    /// **Note**: This method returns a [`NewSpan`] and as such, this
    /// expectation can only be validated when expecting a new span via
    /// [`MockCollector::new_span`]. It cannot be validated on
    /// [`MockCollector::enter`], [`MockCollector::exit`], or any other
    /// method on [`MockCollector`] that takes an `ExpectedSpan`.
    ///
    /// An _explicit_ parent span is one passed to the `span!` macro in the
    /// `parent:` field. If no `parent:` field is specified, then the span
    /// will have a contextually determined parent or be a contextual root if
    /// there is no parent.
    ///
    /// If the ancestry is different from the provided one, this expectation
    /// will fail.
    ///
    /// # Examples
    ///
    /// If `expect::has_explicit_parent("parent_name")` is passed
    /// `with_ancestry` then the provided string is the name of the explicit
    /// parent span to expect.
    ///
    /// ```
    /// use tracing_mock::{collector, expect};
    ///
    /// let span = expect::span()
    ///     .with_ancestry(expect::has_explicit_parent("parent_span"));
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
    ///     .with_ancestry(expect::is_explicit_root());
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
    /// When `expect::has_contextual_parent("parent_name")` is passed to
    /// `with_ancestry` then the provided string is the name of the contextual
    /// parent span to expect.
    ///
    /// ```
    /// use tracing_mock::{collector, expect};
    ///
    /// let parent_span = expect::span().named("parent_span");
    /// let span = expect::span()
    ///     .with_ancestry(expect::has_contextual_parent("parent_span"));
    ///
    /// let (collector, handle) = collector::mock()
    ///     .new_span(&parent_span)
    ///     .enter(&parent_span)
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
    ///     .with_ancestry(expect::is_contextual_root());
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
    /// span is *contextually*—as opposed to explicitly—within the span
    /// `parent_span`:
    ///
    /// ```should_panic
    /// use tracing_mock::{collector, expect};
    ///
    /// let parent_span = expect::span().named("parent_span");
    /// let span = expect::span()
    ///     .with_ancestry(expect::has_explicit_parent("parent_span"));
    ///
    /// let (collector, handle) = collector::mock()
    ///     .new_span(&parent_span)
    ///     .enter(&parent_span)
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
    pub fn with_ancestry(self, ancestry: Ancestry) -> NewSpan {
        NewSpan {
            ancestry: Some(ancestry),
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

        if let Some(expected_id) = &self.id {
            expected_id.check(actual.id(), format_args!("span `{}`", name), collector_name);
        }

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

impl<S> From<S> for NewSpan
where
    S: Into<ExpectedSpan>,
{
    fn from(span: S) -> Self {
        Self {
            span: span.into(),
            ..Default::default()
        }
    }
}

impl NewSpan {
    /// Configures this `NewSpan` to expect the specified [`Ancestry`]. A
    /// span's ancestry indicates whether it has a parent or is a root span
    /// and whether the parent is explitly or contextually assigned.
    ///
    /// For more information and examples, see the documentation on
    /// [`ExpectedSpan::with_ancestry`].
    pub fn with_ancestry(self, ancestry: Ancestry) -> NewSpan {
        NewSpan {
            ancestry: Some(ancestry),
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
        get_ancestry: impl FnOnce() -> Ancestry,
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

        if let Some(ref expected_ancestry) = self.ancestry {
            let actual_ancestry = get_ancestry();
            expected_ancestry.check(
                &actual_ancestry,
                format_args!("span `{}`", name),
                collector_name,
            );
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

        if let Some(ref parent) = self.ancestry {
            s.field("parent", &format_args!("{:?}", parent));
        }

        if !self.fields.is_empty() {
            s.field("fields", &self.fields);
        }

        s.finish()
    }
}

impl PartialEq for ExpectedId {
    fn eq(&self, other: &Self) -> bool {
        self.inner.load(Ordering::Relaxed) == other.inner.load(Ordering::Relaxed)
    }
}

impl Eq for ExpectedId {}

impl ExpectedId {
    const UNSET: u64 = 0;

    pub(crate) fn new_unset() -> Self {
        Self {
            inner: Arc::new(AtomicU64::from(Self::UNSET)),
        }
    }

    pub(crate) fn set(&self, span_id: u64) -> Result<(), SetActualSpanIdError> {
        self.inner
            .compare_exchange(Self::UNSET, span_id, Ordering::Relaxed, Ordering::Relaxed)
            .map_err(|current| SetActualSpanIdError {
                previous_span_id: current,
                new_span_id: span_id,
            })?;
        Ok(())
    }

    pub(crate) fn check(&self, actual: u64, ctx: fmt::Arguments<'_>, collector_name: &str) {
        let id = self.inner.load(Ordering::Relaxed);

        assert!(
            id != Self::UNSET,
            "\n[{}] expected {} to have expected ID set, but it hasn't been, \
            perhaps this `ExpectedId` wasn't used in a call to `MockCollector::new_span()`?",
            collector_name,
            ctx,
        );

        assert_eq!(
            id, actual,
            "\n[{}] expected {} to have ID `{}`, but it has `{}` instead",
            collector_name, ctx, id, actual,
        );
    }
}

#[derive(Debug)]
pub(crate) struct SetActualSpanIdError {
    previous_span_id: u64,
    new_span_id: u64,
}

impl fmt::Display for SetActualSpanIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Could not set `ExpecedId` to {new}, \
            it had already been set to {previous}",
            new = self.new_span_id,
            previous = self.previous_span_id
        )
    }
}

impl error::Error for SetActualSpanIdError {}
