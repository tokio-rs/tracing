mod support;

use self::support::*;
use tracing::Level;

#[test]
fn multiple_max_level_hints() {
    fn do_events() {
        tracing::info!("doing a thing that you might care about");
        tracing::debug!("charging turboencabulator with interocitor");
        tracing::warn!("extremely serious warning, pay attention");
        tracing::trace!("interocitor charge level is 10%");
        tracing::error!("everything is on fire");
    }

    let (subscriber1, handle1) = subscriber::mock()
        .with_max_level_hint(Level::INFO)
        .with_filter(|meta| {
            assert!(
                dbg!(meta).level() <= &Level::DEBUG,
                "a TRACE event was dynamically filtered by subscriber1: "
            );
            true
        })
        .event(event::mock().at_level(Level::INFO))
        .event(event::mock().at_level(Level::WARN))
        .event(event::mock().at_level(Level::ERROR))
        .done()
        .run_with_handle();
    let (subscriber2, handle2) = subscriber::mock()
        .with_max_level_hint(Level::INFO)
        .with_filter(|meta| {
            assert!(
                dbg!(meta).level() <= &Level::DEBUG,
                "a TRACE event was dynamically filtered by subscriber2: "
            );
            true
        })
        .event(event::mock().at_level(Level::TRACE))
        .event(event::mock().at_level(Level::INFO))
        .event(event::mock().at_level(Level::WARN))
        .event(event::mock().at_level(Level::ERROR))
        .done()
        .run_with_handle();

    let dispatch1 = tracing::Dispatch::new(subscriber1);
    let dispatch2 = tracing::Dispatch::new(subscriber2);

    tracing::dispatcher::with_default(&dispatch1, do_events);
    handle1.assert_finished();
    tracing::dispatcher::with_default(&dispatch2, do_events);
    handle2.assert_finished();
}
