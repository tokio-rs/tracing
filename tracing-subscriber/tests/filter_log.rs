mod support;
use self::support::*;
use tracing::{self, collector::with_default, Level};
use tracing_subscriber::{filter::EnvFilter, prelude::*};

#[test]
fn log_is_enabled() {
    mod my_module {
        pub(crate) fn do_test() {
            log::trace!("this should be disabled");
            log::info!("this shouldn't be");
            log::debug!("this should be disabled");
            log::warn!("this should be enabled");
            log::warn!(target: "something else", "this shouldn't be enabled");
            log::error!("this should be enabled too");
        }
    }
    tracing_log::LogTracer::init().expect("logger should be unset");
    let filter: EnvFilter = "filter_log::my_module=info"
        .parse()
        .expect("filter should parse");
    let (subscriber, finished) = collector::mock()
        .event(event::mock().at_level(Level::INFO))
        .event(event::mock().at_level(Level::WARN))
        .event(event::mock().at_level(Level::ERROR))
        .done()
        .run_with_handle();
    let subscriber = subscriber.with(filter);

    with_default(subscriber, || {
        my_module::do_test();
    });

    finished.assert_finished();
}
