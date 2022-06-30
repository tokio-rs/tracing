mod support;
use self::support::*;
use tracing::Collect;
use tracing::{level_filters::LevelFilter, Level};
use tracing_subscriber::{filter, prelude::*, Subscribe};

#[test]
fn just_empty_vec() {
    // Just a None means everything is off
    let collector = tracing_subscriber::registry().with(Vec::<ExpectSubscriber>::new());
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::OFF));
}

#[test]
fn subscriber_and_empty_vec() {
    let info = subscriber::named("info")
        .run()
        .with_filter(LevelFilter::INFO)
        .boxed();

    let collector = tracing_subscriber::registry()
        .with(info)
        .with(Vec::<ExpectSubscriber>::new());
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::INFO));
}
