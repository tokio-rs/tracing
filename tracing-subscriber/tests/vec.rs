mod support;
use self::support::*;
use tracing::Collect;
use tracing::{level_filters::LevelFilter, Level};
use tracing_subscriber::{filter, prelude::*, Subscribe};

#[test]
fn just_empty_vec() {
    // Just a None means everything is off
    let collector = tracing_subscriber::registry().with(Vec::<LevelFilter>::new());
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::OFF));
}

#[test]
fn subscriber_and_empty_vec() {
    let collector = tracing_subscriber::registry()
        .with(LevelFilter::INFO)
        .with(Vec::<LevelFilter>::new());
    assert_eq!(collector.max_level_hint(), Some(LevelFilter::INFO));
}
