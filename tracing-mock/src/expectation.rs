use crate::{
    event::MockEvent,
    field,
    span::{MockSpan, NewSpan},
};

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Expect {
    Event(MockEvent),
    FollowsFrom {
        consequence: MockSpan,
        cause: MockSpan,
    },
    Enter(MockSpan),
    Exit(MockSpan),
    CloneSpan(MockSpan),
    DropSpan(MockSpan),
    Visit(MockSpan, field::Expect),
    NewSpan(NewSpan),
    Nothing,
}
