#![cfg(all(feature = "env-filter", feature = "tracing-log"))]
mod support;
use self::support::*;
use tracing::{self, Level};
use tracing_subscriber::{filter::EnvFilter, prelude::*};

mod my_module {
    pub(crate) fn test_records() {
        log::trace!("this should be disabled");
        log::info!("this shouldn't be");
        log::debug!("this should be disabled");
        log::warn!("this should be enabled");
        log::warn!(target: "something else", "this shouldn't be enabled");
        log::error!("this should be enabled too");
    }

    pub(crate) fn test_log_enabled() {
        assert!(
            log::log_enabled!(log::Level::Info),
            "info should be enabled inside `my_module`"
        );
        assert!(
            !log::log_enabled!(log::Level::Debug),
            "debug should not be enabled inside `my_module`"
        );
        assert!(
            log::log_enabled!(log::Level::Warn),
            "warn should be enabled inside `my_module`"
        );
    }
}

#[test]
fn log_is_enabled() {
    let filter: EnvFilter = "filter_log::my_module=info"
        .parse()
        .expect("filter should parse");
    let (collector, finished) = collector::mock()
        .event(event::mock().at_level(Level::INFO))
        .event(event::mock().at_level(Level::WARN))
        .event(event::mock().at_level(Level::ERROR))
        .done()
        .run_with_handle();

    // Note: we have to set the global default in order to set the `log` max
    // level, which can only be set once.
    collector.with(filter).init();

    my_module::test_records();
    log::info!("this is disabled");

    my_module::test_log_enabled();
    assert!(
        !log::log_enabled!(log::Level::Info),
        "info should not be enabled outside `my_module`"
    );
    assert!(
        !log::log_enabled!(log::Level::Warn),
        "warn should not be enabled outside `my_module`"
    );

    finished.assert_finished();
}
