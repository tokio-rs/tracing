use tracing::{collect::with_default, Level};
use tracing_mock::{expect, subscriber};
use tracing_subscriber::{filter, reload, Subscribe};

#[test]
fn reload_filter() {
    let filter = filter::LevelFilter::INFO;
    let (mock_subscriber, mock_handle) = subscriber::mock()
        .event(expect::event().at_level(Level::INFO).named("before 1"))
        .event(expect::event().at_level(Level::WARN).named("before 2"))
        .event(expect::event().at_level(Level::WARN).named("after 2"))
        .only()
        .run_with_handle();

    let filtered_subscriber = mock_subscriber.with_filter(filter);
    let (reload_layer, reload_handle) = reload::Subscriber::new(filtered_subscriber);
    let collector = reload_layer.with_collector(tracing_subscriber::registry());

    with_default(collector, || {
        tracing::info!(name: "before 1", "");
        tracing::warn!(name: "before 2", "");
        reload_handle
            .modify(|layer| {
                *layer.filter_mut() = filter::LevelFilter::WARN;
            })
            .unwrap();
        tracing::info!(name: "after 1", "");
        tracing::warn!(name: "after 2", "");
    });

    mock_handle.assert_finished();
}
