mod support;
use self::support::*;
use tracing::{self, Level};
use tracing_subscriber::{filter::LevelFilter, prelude::*, reload};

#[test]
fn reload_max_log_level() {
    let (collector, finished) = collector::mock()
        .event(event::mock().at_level(Level::INFO))
        .event(event::mock().at_level(Level::DEBUG))
        .event(event::mock().at_level(Level::INFO))
        .done()
        .run_with_handle();
    let (filter, reload_handle) = reload::Subscriber::new(LevelFilter::INFO);
    collector.with(filter).init();

    assert!(log::log_enabled!(log::Level::Info));
    assert!(!log::log_enabled!(log::Level::Debug));
    assert!(!log::log_enabled!(log::Level::Trace));

    log::debug!("i'm disabled");
    log::info!("i'm enabled");

    reload_handle
        .reload(Level::DEBUG)
        .expect("reloading succeeds");

    assert!(log::log_enabled!(log::Level::Info));
    assert!(log::log_enabled!(log::Level::Debug));
    assert!(!log::log_enabled!(log::Level::Trace));

    log::debug!("i'm enabled now");
    log::info!("i'm still enabled, too");

    finished.assert_finished();
}
