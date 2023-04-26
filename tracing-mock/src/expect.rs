//! Construct expectations for traces which should be received
//!
//! This module contains constructors for expectations defined
//! in the [`event`], [`span`], and [`field`] modules.
//!
//! # Examples
//!
//! ```
//! use tracing_mock::{collector, expect};
//!
//! let (collector, handle) = collector::mock()
//!     // Expect an event with message
//!     .event(expect::event().with_fields(expect::message("message")))
//!     .only()
//!     .run_with_handle();
//!
//! tracing::collect::with_default(collector, || {
//!     tracing::info!("message");
//! });
//!
//! handle.assert_finished();
//! ```
use crate::{
    event::ExpectedEvent,
    field::{ExpectedField, ExpectedFields, ExpectedValue},
    span::{ExpectedSpan, NewSpan},
};

use std::fmt;

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
/// use tracing_mock::{collector, expect};
///
/// let (collector, handle) = collector::mock()
///     .event(expect::event())
///     .run_with_handle();
///
/// tracing::collect::with_default(collector, || {
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
/// use tracing_mock::{collector, expect};
///
/// let (collector, handle) = collector::mock()
///     .event(expect::event())
///     .run_with_handle();
///
/// tracing::collect::with_default(collector, || {
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
/// use tracing_mock::{collector, expect};
///
/// let (collector, handle) = collector::mock()
///     .new_span(expect::span())
///     .enter(expect::span())
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
/// If we expect an event and instead record something else, the test
/// will fail:
///
/// ```should_panic
/// use tracing_mock::{collector, expect};
///
/// let (collector, handle) = collector::mock()
///     .enter(expect::span())
///     .run_with_handle();
///
/// tracing::collect::with_default(collector, || {
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
/// use tracing_mock::{collector, expect};
///
/// let event = expect::event()
///     .with_fields(expect::field("field.name").with_value(&"field_value"));
///
/// let (collector, handle) = collector::mock()
///     .event(event)
///     .run_with_handle();
///
/// tracing::collect::with_default(collector, || {
///     tracing::info!(field.name = "field_value");
/// });
///
/// handle.assert_finished();
/// ```
///
/// A different field value will cause the test to fail:
///
/// ```should_panic
/// use tracing_mock::{collector, expect};
///
/// let event = expect::event()
///     .with_fields(expect::field("field.name").with_value(&"field_value"));
///
/// let (collector, handle) = collector::mock()
///     .event(event)
///     .run_with_handle();
///
/// tracing::collect::with_default(collector, || {
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
/// hwo to expect multiple fields, see the [`field`] module and the
/// [`ExpectedField`] and [`ExpectedFields`] structs.
/// span, see the [`span`] module and the [`ExpectedSpan`] and
/// [`NewSpan`] structs.
///
/// This is equivalent to
/// `expect::field("message").with_value(message)`.
///
/// # Examples
///
/// ```
/// use tracing_mock::{collector, expect};
///
/// let event = expect::event().with_fields(
///     expect::message("message"));
///
/// let (collector, handle) = collector::mock()
///     .event(event)
///     .run_with_handle();
///
/// tracing::collect::with_default(collector, || {
///     tracing::info!("message");
/// });
///
/// handle.assert_finished();
/// ```
///
/// A different message value will cause the test to fail:
///
/// ```should_panic
/// use tracing_mock::{collector, expect};
///
/// let event = expect::event().with_fields(
///     expect::message("message"));
///
/// let (collector, handle) = collector::mock()
///     .event(event)
///     .run_with_handle();
///
/// tracing::collect::with_default(collector, || {
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
            Expect::Nothing => panic!(
                "\n[{}] expected nothing else to happen\n[{}] but {} instead",
                name, name, what,
            ),
        }
    }
}
