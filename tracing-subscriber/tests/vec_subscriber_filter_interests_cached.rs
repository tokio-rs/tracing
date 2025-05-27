#![cfg(feature = "registry")]
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tracing::{Collect, Level};
use tracing_mock::{
    expect,
    subscriber::{self, MockSubscriber},
};
use tracing_subscriber::{filter, prelude::*};

#[test]
fn vec_subscriber_filter_interests_are_cached() {
    let mk_filtered = |level: Level, subscriber: MockSubscriber| {
        let seen = Arc::new(Mutex::new(HashMap::new()));
        let filter = filter::filter_fn({
            let seen = seen.clone();
            move |meta| {
                *seen.lock().unwrap().entry(*meta.level()).or_insert(0usize) += 1;
                meta.level() <= &level
            }
        });
        (subscriber.with_filter(filter).boxed(), seen)
    };

    // This subscriber will return Interest::always for INFO and lower.
    let (info_subscriber, info_handle) = subscriber::named("info")
        .event(expect::event().at_level(Level::INFO))
        .event(expect::event().at_level(Level::WARN))
        .event(expect::event().at_level(Level::ERROR))
        .event(expect::event().at_level(Level::INFO))
        .event(expect::event().at_level(Level::WARN))
        .event(expect::event().at_level(Level::ERROR))
        .only()
        .run_with_handle();
    let (info_subscriber, seen_info) = mk_filtered(Level::INFO, info_subscriber);

    // This subscriber will return Interest::always for WARN and lower.
    let (warn_subscriber, warn_handle) = subscriber::named("warn")
        .event(expect::event().at_level(Level::WARN))
        .event(expect::event().at_level(Level::ERROR))
        .event(expect::event().at_level(Level::WARN))
        .event(expect::event().at_level(Level::ERROR))
        .only()
        .run_with_handle();
    let (warn_subscriber, seen_warn) = mk_filtered(Level::WARN, warn_subscriber);

    let collector = tracing_subscriber::registry().with(vec![warn_subscriber, info_subscriber]);
    assert!(collector.max_level_hint().is_none());

    let _collector = collector.set_default();

    fn events() {
        tracing::trace!("hello trace");
        tracing::debug!("hello debug");
        tracing::info!("hello info");
        tracing::warn!("hello warn");
        tracing::error!("hello error");
    }

    events();
    {
        let lock = seen_info.lock().unwrap();
        for (&level, &count) in lock.iter() {
            if level == Level::INFO {
                continue;
            }
            assert_eq!(
                count, 1,
                "level {level:?} should have been seen 1 time by the INFO subscriber (after first set of events)"
            );
        }

        let lock = seen_warn.lock().unwrap();
        for (&level, &count) in lock.iter() {
            if level == Level::INFO {
                continue;
            }
            assert_eq!(
                count, 1,
                "level {level:?} should have been seen 1 time by the WARN subscriber (after first set of events)"
            );
        }
    }

    events();
    {
        let lock = seen_info.lock().unwrap();
        for (&level, &count) in lock.iter() {
            if level == Level::INFO {
                continue;
            }
            assert_eq!(
                count, 1,
                "level {level:?} should have been seen 1 time by the INFO subscriber (after second set of events)"
            );
        }

        let lock = seen_warn.lock().unwrap();
        for (&level, &count) in lock.iter() {
            if level == Level::INFO {
                continue;
            }
            assert_eq!(
                count, 1,
                "level {level:?} should have been seen 1 time by the WARN subscriber (after second set of events)"
            );
        }
    }

    info_handle.assert_finished();
    warn_handle.assert_finished();
}
