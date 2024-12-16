#![cfg(feature = "registry")]
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tracing::{Collect, Level};
use tracing_mock::{expect, subscriber};
use tracing_subscriber::{filter, prelude::*};

#[test]
fn multiple_subscriber_filter_interests_are_cached() {
    // This subscriber will return Interest::always for INFO and lower.
    let seen_info = Arc::new(Mutex::new(HashMap::new()));
    let seen_info2 = seen_info.clone();
    let filter = filter::filter_fn(move |meta| {
        *seen_info
            .lock()
            .unwrap()
            .entry(*meta.level())
            .or_insert(0usize) += 1;
        meta.level() <= &Level::INFO
    });
    let seen_info = seen_info2;

    let (info_subscriber, info_handle) = subscriber::named("info")
        .event(expect::event().at_level(Level::INFO))
        .event(expect::event().at_level(Level::WARN))
        .event(expect::event().at_level(Level::ERROR))
        .event(expect::event().at_level(Level::INFO))
        .event(expect::event().at_level(Level::WARN))
        .event(expect::event().at_level(Level::ERROR))
        .only()
        .run_with_handle();
    let info_subscriber = info_subscriber.with_filter(filter);

    // This subscriber will return Interest::always for WARN and lower.
    let seen_warn = Arc::new(Mutex::new(HashMap::new()));
    let seen_warn2 = seen_warn.clone();
    let filter = filter::filter_fn(move |meta| {
        *seen_warn
            .lock()
            .unwrap()
            .entry(*meta.level())
            .or_insert(0usize) += 1;
        meta.level() <= &Level::WARN
    });
    let seen_warn = seen_warn2;

    let (warn_subscriber, warn_handle) = subscriber::named("warn")
        .event(expect::event().at_level(Level::WARN))
        .event(expect::event().at_level(Level::ERROR))
        .event(expect::event().at_level(Level::WARN))
        .event(expect::event().at_level(Level::ERROR))
        .only()
        .run_with_handle();
    let warn_subscriber = warn_subscriber.with_filter(filter);

    let subscriber = tracing_subscriber::registry()
        .with(warn_subscriber)
        .with(info_subscriber);
    assert!(subscriber.max_level_hint().is_none());

    let _subscriber = subscriber.set_default();

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
