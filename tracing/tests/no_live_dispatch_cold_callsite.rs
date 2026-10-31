#![cfg(feature = "std")]

use tracing::{
    callsite::DefaultCallsite, metadata::Kind, subscriber::NoSubscriber, Dispatch, Level,
};

#[test]
fn cold_callsite_with_no_live_dispatch_is_never() {
    static __CALLSITE: DefaultCallsite = tracing::callsite2! {
        name: "unwatched",
        kind: Kind::EVENT,
        level: Level::INFO,
        fields:
    };

    drop(Dispatch::new(NoSubscriber::default()));

    assert!(__CALLSITE.interest().is_never());
}
