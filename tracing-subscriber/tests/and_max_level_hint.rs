#![cfg(feature = "registry")]
use tracing::Subscriber;
use tracing_subscriber::{
    filter::{filter_fn, FilterExt, LevelFilter},
    layer::Identity,
    prelude::*,
};

// https://github.com/tokio-rs/tracing/issues/3640
#[test]
fn and_with_an_unhinted_filter_keeps_the_level_hint() {
    let filter = LevelFilter::INFO.and(filter_fn(|meta| meta.is_event()));
    let subscriber = tracing_subscriber::registry().with(Identity::new().with_filter(filter));
    assert_eq!(subscriber.max_level_hint(), Some(LevelFilter::INFO));
}
