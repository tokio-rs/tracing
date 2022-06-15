#![cfg(feature = "registry")]

use std::sync::{Arc, Mutex};
use tracing::{collect::with_default, Collect};
use tracing_subscriber::{prelude::*, registry, Subscribe};

struct TrackingLayer {
    event_enabled_count: Arc<Mutex<usize>>,
}

impl<C> Subscribe<C> for TrackingLayer
where
    C: Collect + Send + Sync + 'static,
{
    fn event_enabled(
        &self,
        _event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::subscribe::Context<'_, C>,
    ) -> bool {
        *self.event_enabled_count.lock().unwrap() += 1;
        true
    }
}

#[test]
fn event_enabled_is_only_called_once() {
    let event_enabled_count = Arc::new(Mutex::default());
    let count = event_enabled_count.clone();
    let collector = registry().with(TrackingLayer {
        event_enabled_count,
    });
    with_default(collector, || {
        tracing::error!("hiya!");
    });

    assert_eq!(1, *count.lock().unwrap());
}
