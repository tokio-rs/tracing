//! Construct expectations for traces which should be received
//!
//! This module contains constructors for expectations defined
//! in the [`event`], [`span`], and [`field`] modules.
//!
//! # Examples
//!
//! ```
//! use tracing_mock::{expect, subscriber};
//!
//! let (subscriber, handle) = subscriber::mock()
//!     // Expect an event with message
//!     .event(expect::event().with_fields(expect::msg("message")))
//!     .only()
//!     .run_with_handle();
//!
//! tracing::subscriber::with_default(subscriber, || {
//!     tracing::info!("message");
//! });
//!
//! handle.assert_finished();
//! ```
use std::fmt;

use crate::{
    ancestry::ExpectedAncestry,
    event::ExpectedEvent,
    field::{ExpectedField, ExpectedFields, ExpectedValue},
    span::{ExpectedId, ExpectedSpan, NewSpan},
};

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Expect {
    Event(ExpectedEvent),
    FollowsFrom {
        consequence: ExpectedSpan,
        cause: ExpectedSpan,
    },
    Enter(ExpectedSpan),
    Exit(ExpectedSpan),
    CloneSpan(ExpectedSpan),
    DropSpan(ExpectedSpan),
    Visit(ExpectedSpan, ExpectedFields),
    NewSpan(NewSpan),
    OnRegisterDispatch,
    Nothing,
}

/// Create a new [`ExpectedEvent`].
///
/// For details on how to add additional assertions to the expected
/// event, see the [`event`] module and the [`ExpectedEvent`] struct.
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
///     tracing::info!(field.name = "field_value");
/// });
///
/// handle.assert_finished();
/// ```
///
/// If we expect an event and instead record something else, the test
/// will fail:
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
/// });
///
/// handle.assert_finished();
/// ```
pub fn event() -> ExpectedEvent {
    ExpectedEvent {
        ..Default::default()
    }
}

/// Construct a new [`ExpectedSpan`].
///
/// For details on how to add additional assertions to the expected
/// span, see the [`span`] module and the [`ExpectedSpan`] and
/// [`NewSpan`] structs.
///
/// # Examples
///
/// ```
/// use tracing_mock::{expect, subscriber};
///
/// let (subscriber, handle) = subscriber::mock()
///     .new_span(expect::span())
///     .enter(expect::span())
///     .run_with_handle();
///
/// tracing::subscriber::with_default(subscriber, || {
///     let span = tracing::info_span!("span");
///     let _guard = span.enter();
/// });
///
/// handle.assert_finished();
/// ```
///
/// If we expect to enter a span and instead record something else, the test
/// will fail:
///
/// ```should_panic
/// use tracing_mock::{expect, subscriber};
///
/// let (subscriber, handle) = subscriber::mock()
///     .enter(expect::span())
///     .run_with_handle();
///
/// tracing::subscriber::with_default(subscriber, || {
///     tracing::info!(field.name = "field_value");
/// });
///
/// handle.assert_finished();
/// ```
pub fn span() -> ExpectedSpan {
    ExpectedSpan {
        ..Default::default()
    }
}

/// Construct a new [`ExpectedField`].
///
/// For details on how to set the value of the expected field and
/// how to expect multiple fields, see the [`field`] module and the
/// [`ExpectedField`] and [`ExpectedFields`] structs.
/// span, see the [`span`] module and the [`ExpectedSpan`] and
/// [`NewSpan`] structs.
///
/// # Examples
///
/// ```
/// use tracing_mock::{expect, subscriber};
///
/// let event = expect::event()
///     .with_fields(expect::field("field.name").with_value(&"field_value"));
///
/// let (subscriber, handle) = subscriber::mock()
///     .event(event)
///     .run_with_handle();
///
/// tracing::subscriber::with_default(subscriber, || {
///     tracing::info!(field.name = "field_value");
/// });
///
/// handle.assert_finished();
/// ```
///
/// A different field value will cause the test to fail:
///
/// ```should_panic
/// use tracing_mock::{expect, subscriber};
///
/// let event = expect::event()
///     .with_fields(expect::field("field.name").with_value(&"field_value"));
///
/// let (subscriber, handle) = subscriber::mock()
///     .event(event)
///     .run_with_handle();
///
/// tracing::subscriber::with_default(subscriber, || {
///     tracing::info!(field.name = "different_field_value");
/// });
///
/// handle.assert_finished();
/// ```
pub fn field<K>(name: K) -> ExpectedField
where
    String: From<K>,
{
    ExpectedField {
        name: name.into(),
        value: ExpectedValue::Any,
    }
}

/// Construct a new message [`ExpectedField`].
///
/// For details on how to set the value of the message field and
/// how to expect multiple fields, see the [`field`] module and the
/// [`ExpectedField`] and [`ExpectedFields`] structs.
///
/// This is equivalent to
/// `expect::field("message").with_value(message)`.
///
/// # Examples
///
/// ```
/// use tracing_mock::{expect, subscriber};
///
/// let event = expect::event().with_fields(
///     expect::msg("message"));
///
/// let (subscriber, handle) = subscriber::mock()
///     .event(event)
///     .run_with_handle();
///
/// tracing::subscriber::with_default(subscriber, || {
///     tracing::info!("message");
/// });
///
/// handle.assert_finished();
/// ```
///
/// A different message value will cause the test to fail:
///
/// ```should_panic
/// use tracing_mock::{expect, subscriber};
///
/// let event = expect::event().with_fields(
///     expect::msg("message"));
///
/// let (subscriber, handle) = subscriber::mock()
///     .event(event)
///     .run_with_handle();
///
/// tracing::subscriber::with_default(subscriber, || {
///     tracing::info!("different message");
/// });
///
/// handle.assert_finished();
/// ```
pub fn msg(message: impl fmt::Display) -> ExpectedField {
    ExpectedField {
        name: "message".to_string(),
        value: ExpectedValue::Debug(message.to_string()),
    }
}

/// Returns a new, unset `ExpectedId`.
///
/// The `ExpectedId` needs to be attached to a [`NewSpan`] or an
/// [`ExpectedSpan`] passed to [`MockSubscriber::new_span`] to
/// ensure that it gets set. When the a clone of the same
/// `ExpectedSpan` is attached to an [`ExpectedSpan`] and passed to
/// any other method on [`MockSubscriber`] that accepts it, it will
/// ensure that it is exactly the same span used across those
/// distinct expectations.
///
/// For more details on how to use this struct, see the documentation
/// on [`ExpectedSpan::with_id`].
///
/// [`MockSubscriber`]: struct@crate::subscriber::MockSubscriber
/// [`MockSubscriber::new_span`]: fn@crate::subscriber::MockSubscriber::new_span
pub fn id() -> ExpectedId {
    ExpectedId::new_unset()
}

/// Convenience function that returns [`ExpectedAncestry::IsContextualRoot`].
pub fn is_contextual_root() -> ExpectedAncestry {
    ExpectedAncestry::IsContextualRoot
}

/// Convenience function that returns [`ExpectedAncestry::HasContextualParent`] with
/// provided name.
pub fn has_contextual_parent<S: Into<ExpectedSpan>>(span: S) -> ExpectedAncestry {
    ExpectedAncestry::HasContextualParent(span.into())
}

/// Convenience function that returns [`ExpectedAncestry::IsExplicitRoot`].
pub fn is_explicit_root() -> ExpectedAncestry {
    ExpectedAncestry::IsExplicitRoot
}

/// Convenience function that returns [`ExpectedAncestry::HasExplicitParent`] with
/// provided name.
pub fn has_explicit_parent<S: Into<ExpectedSpan>>(span: S) -> ExpectedAncestry {
    ExpectedAncestry::HasExplicitParent(span.into())
}

impl Expect {
    pub(crate) fn bad(&self, name: impl AsRef<str>, what: fmt::Arguments<'_>) {
        let name = name.as_ref();
        match self {
            Expect::Event(e) => panic!(
                "\n[{}] expected event {}\n[{}] but instead {}",
                name, e, name, what,
            ),
            Expect::FollowsFrom { consequence, cause } => panic!(
                "\n[{}] expected consequence {} to follow cause {} but instead {}",
                name, consequence, cause, what,
            ),
            Expect::Enter(e) => panic!(
                "\n[{}] expected to enter {}\n[{}] but instead {}",
                name, e, name, what,
            ),
            Expect::Exit(e) => panic!(
                "\n[{}] expected to exit {}\n[{}] but instead {}",
                name, e, name, what,
            ),
            Expect::CloneSpan(e) => {
                panic!(
                    "\n[{}] expected to clone {}\n[{}] but instead {}",
                    name, e, name, what,
                )
            }
            Expect::DropSpan(e) => {
                panic!(
                    "\n[{}] expected to drop {}\n[{}] but instead {}",
                    name, e, name, what,
                )
            }
            Expect::Visit(e, fields) => panic!(
                "\n[{}] expected {} to record {}\n[{}] but instead {}",
                name, e, fields, name, what,
            ),
            Expect::NewSpan(e) => panic!(
                "\n[{}] expected {}\n[{}] but instead {}",
                name, e, name, what
            ),
            Expect::OnRegisterDispatch => panic!(
                "\n[{}] expected on_register_dispatch to be called\n[{}] but instead {}",
                name, name, what
            ),
            Expect::Nothing => panic!(
                "\n[{}] expected nothing else to happen\n[{}] but {} instead",
                name, name, what,
            ),
        }
    }
}
