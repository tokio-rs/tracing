use crate::{
    event::ExpectedEvent,
    field::{ExpectedField, ExpectedFields, ExpectedValue},
    span::{ExpectedSpan, NewSpan},
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

pub fn span() -> ExpectedSpan {
    ExpectedSpan {
        ..Default::default()
    }
}
