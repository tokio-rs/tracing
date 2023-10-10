use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering::SeqCst},
};

use tracing::{Event, Metadata};
use tracing_subscriber::{
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
};

#[derive(Clone, Debug, Default)]
struct MyLayer {
    callsite_registered: Arc<AtomicBool>,
}

impl<S> tracing_subscriber::Layer<S> for MyLayer
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn register_callsite(&self, _m: &'static Metadata<'static>) -> tracing::subscriber::Interest {
        std::thread::sleep(std::time::Duration::from_millis(100)); // Simulate some work
        self.callsite_registered.store(true, SeqCst);
        tracing::subscriber::Interest::always()
    }

    fn on_event(&self, _event: &Event<'_>, _ctx: Context<'_, S>) {
        assert!(self.callsite_registered.load(SeqCst));
    }
}

#[test]
fn missed_register_callsite() {
    let my_layer = MyLayer::default();
    tracing_subscriber::registry().with(my_layer.clone()).init();

    std::thread::scope(|s| {
        for i in 0..16 {
            s.spawn(move || tracing::info!("Thread {} started", i));
        }
    });

    dbg!(my_layer);
}
