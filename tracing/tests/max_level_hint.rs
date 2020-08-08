mod support;

use self::support::*;
use tracing::Level;

#[test]
fn max_level_hints() {
    let (subscriber, handle) = subscriber::mock()
        .with_max_level_hint(Level::INFO)
        .with_filter(|meta| {
            assert!(
                dbg!(meta).level() <= &Level::INFO,
                "a TRACE or DEBUG event was dynamically filtered: "
            );
            true
        })
        .event(event::mock().at_level(Level::INFO))
        .event(event::mock().at_level(Level::WARN))
        .event(event::mock().at_level(Level::ERROR))
        .done()
        .run_with_handle();

    tracing::subscriber::set_global_default(subscriber).unwrap();

    tracing::info!("doing a thing that you might care about");
    tracing::debug!("charging turboencabulator with interocitor");
    tracing::warn!("extremely serious warning, pay attention");
    tracing::trace!("interocitor charge level is 10%");
    tracing::error!("everything is on fire");
    handle.assert_finished();
}
