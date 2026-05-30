#![cfg(feature = "std")]
mod common;

use common::*;
use tracing_core::{
    callsite, dispatcher::*, field, metadata::Metadata, span, subscriber::Subscriber, Event, Kind,
    Level,
};

#[test]
fn set_default_dispatch() {
    set_global_default(Dispatch::new(TestSubscriberA)).expect("global dispatch set failed");
    get_default(|current| {
        assert!(
            current.is::<TestSubscriberA>(),
            "global dispatch get failed"
        )
    });

    let guard = set_default(&Dispatch::new(TestSubscriberB));
    get_default(|current| assert!(current.is::<TestSubscriberB>(), "set_default get failed"));

    // Drop the guard, setting the dispatch back to the global dispatch
    drop(guard);

    get_default(|current| {
        assert!(
            current.is::<TestSubscriberA>(),
            "global dispatch get failed"
        )
    });
}

#[test]
fn nested_set_default() {
    let _guard = set_default(&Dispatch::new(TestSubscriberA));
    get_default(|current| {
        assert!(
            current.is::<TestSubscriberA>(),
            "set_default for outer subscriber failed"
        )
    });

    let inner_guard = set_default(&Dispatch::new(TestSubscriberB));
    get_default(|current| {
        assert!(
            current.is::<TestSubscriberB>(),
            "set_default inner subscriber failed"
        )
    });

    drop(inner_guard);
    get_default(|current| {
        assert!(
            current.is::<TestSubscriberA>(),
            "set_default outer subscriber failed"
        )
    });
}

#[test]
fn interest_cache_rebuilt_when_default_dropped() {
    static CALLSITE: callsite::DefaultCallsite = {
        // The values of the metadata are unimportant
        static META: Metadata<'static> = Metadata::new(
            "event ",
            "module::path",
            Level::INFO,
            None,
            None,
            None,
            field::FieldSet::new(&[], callsite::Identifier(&CALLSITE)),
            Kind::EVENT,
        );
        callsite::DefaultCallsite::new(&META)
    };

    let _guard = set_default(&Dispatch::new(NeverSubscriber));
    assert!(CALLSITE.interest().is_never());

    let inner_guard = set_default(&Dispatch::new(AlwaysSubscriber));
    assert!(CALLSITE.interest().is_sometimes());

    drop(inner_guard);
    assert!(CALLSITE.interest().is_never());
}

pub struct AlwaysSubscriber;
impl Subscriber for AlwaysSubscriber {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }
    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
    fn event(&self, _: &Event<'_>) {}
    fn enter(&self, _: &span::Id) {}
    fn exit(&self, _: &span::Id) {}
}
pub struct NeverSubscriber;
impl Subscriber for NeverSubscriber {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        false
    }
    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }
    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
    fn event(&self, _: &Event<'_>) {}
    fn enter(&self, _: &span::Id) {}
    fn exit(&self, _: &span::Id) {}
}
