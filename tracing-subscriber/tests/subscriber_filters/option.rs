use super::*;
use tracing::Collect;

// This test is just used to compare to the tests below
#[test]
fn just_layer() {
    let info = subscriber::named("info")
        .run()
        .with_filter(LevelFilter::INFO)
        .boxed();

    let collector = tracing_subscriber::registry().with(info);
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::INFO));
}

#[test]
fn layer_and_option_layer() {
    let info = subscriber::named("info")
        .run()
        .with_filter(LevelFilter::INFO)
        .boxed();

    let debug = subscriber::named("debug")
        .run()
        .with_filter(LevelFilter::DEBUG)
        .boxed();

    let mut collector = tracing_subscriber::registry().with(info).with(Some(debug));
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::DEBUG));

    let error = subscriber::named("error")
        .run()
        .with_filter(LevelFilter::ERROR)
        .boxed();
    // None means the other layer takes control
    collector = tracing_subscriber::registry().with(error).with(None);
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::ERROR));
}

#[test]
fn just_option_layer() {
    // Just a None means everything is off
    let collector = tracing_subscriber::registry().with(None::<ExpectSubscriber>);
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::OFF));
}
