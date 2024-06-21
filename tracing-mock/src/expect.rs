use std::fmt;

use crate::{
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
    Nothing,
}

pub fn event() -> ExpectedEvent {
    ExpectedEvent {
        ..Default::default()
    }
}

pub fn field<K>(name: K) -> ExpectedField
where
    String: From<K>,
{
    ExpectedField {
        name: name.into(),
        value: ExpectedValue::Any,
    }
}

pub fn message(message: impl fmt::Display) -> ExpectedField {
    ExpectedField {
        name: "message".to_string(),
        value: ExpectedValue::Debug(message.to_string()),
    }
}

pub fn span() -> ExpectedSpan {
    ExpectedSpan {
        ..Default::default()
    }
}

/// Returns a new, unset `ExpectedId`.
///
/// The `ExpectedId` needs to be attached to a [`NewSpan`] or an
/// [`ExpectedSpan`] passed to [`MockCollector::new_span`] to
/// ensure that it gets set. When the a clone of the same
/// `ExpectedSpan` is attached to an [`ExpectedSpan`] and passed to
/// any other method on [`MockCollector`] that accepts it, it will
/// ensure that it is exactly the same span used across those
/// distinct expectations.
///
/// For more details on how to use this struct, see the documentation
/// on [`ExpectedSpan::with_id`].
///
/// [`MockCollector`]: struct@crate::collector::MockCollector
/// [`MockCollector::new_span`]: fn@crate::collector::MockCollector::new_span
pub fn id() -> ExpectedId {
    ExpectedId::new_unset()
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
