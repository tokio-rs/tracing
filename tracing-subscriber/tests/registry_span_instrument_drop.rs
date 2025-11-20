#![cfg(feature = "registry")]

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use tracing::{
    span::{self, Id},
    Dispatch, Event, Metadata, Subscriber,
};
use tracing_core::{Interest, LevelFilter};
use tracing_subscriber::{
    layer::{Context, SubscriberExt},
    Layer, Registry,
};

#[test]
fn span_entered_on_different_thread_from_subscriber() {
    /// Wraps `tracing_subscriber::Registry` and adds some accounting
    /// to verify that the subscriber is receiving the expected number of calls.
    struct CountingSubscriber {
        inner: Registry,
        new_count: Arc<AtomicUsize>,
        clone_count: Arc<AtomicUsize>,
        enter_count: Arc<AtomicUsize>,
        exit_count: Arc<AtomicUsize>,
        close_count: Arc<AtomicUsize>,
    }

    // Forward all subscriber methods to the inner registry, adding counts where appropriate.
    impl Subscriber for CountingSubscriber {
        fn on_register_dispatch(&self, subscriber: &Dispatch) {
            self.inner.on_register_dispatch(subscriber);
        }

        fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
            self.inner.register_callsite(metadata)
        }

        fn max_level_hint(&self) -> Option<LevelFilter> {
            self.inner.max_level_hint()
        }

        fn event_enabled(&self, event: &Event<'_>) -> bool {
            self.inner.event_enabled(event)
        }

        fn clone_span(&self, id: &span::Id) -> span::Id {
            self.clone_count.fetch_add(1, Ordering::SeqCst);
            self.inner.clone_span(id)
        }

        fn drop_span(&self, id: span::Id) {
            self.close_count.fetch_add(1, Ordering::SeqCst);
            #[allow(deprecated)]
            self.inner.drop_span(id);
        }

        fn try_close(&self, id: span::Id) -> bool {
            self.close_count.fetch_add(1, Ordering::SeqCst);
            self.inner.try_close(id)
        }

        fn current_span(&self) -> tracing_core::span::Current {
            self.inner.current_span()
        }

        unsafe fn downcast_raw(&self, id: std::any::TypeId) -> Option<*const ()> {
            self.inner.downcast_raw(id)
        }

        fn enabled(&self, metadata: &Metadata<'_>) -> bool {
            self.inner.enabled(metadata)
        }

        fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
            self.new_count.fetch_add(1, Ordering::SeqCst);
            self.inner.new_span(span)
        }

        fn record(&self, span: &span::Id, values: &span::Record<'_>) {
            self.inner.record(span, values);
        }

        fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
            self.inner.record_follows_from(span, follows);
        }

        fn event(&self, event: &Event<'_>) {
            self.inner.event(event);
        }

        fn enter(&self, span: &span::Id) {
            self.inner.enter(span);
            self.enter_count.fetch_add(1, Ordering::SeqCst);
        }

        fn exit(&self, span: &span::Id) {
            self.inner.exit(span);
            self.exit_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Similar to the above, but for a `Layer` which sits atop the subscriber.
    struct CountingLayer {
        new_count: Arc<AtomicUsize>,
        enter_count: Arc<AtomicUsize>,
        exit_count: Arc<AtomicUsize>,
        close_count: Arc<AtomicUsize>,
    }

    // Just does bookkeeping where relevant.
    impl Layer<CountingSubscriber> for CountingLayer {
        fn on_new_span(
            &self,
            _attrs: &span::Attributes<'_>,
            _id: &span::Id,
            _ctx: Context<'_, CountingSubscriber>,
        ) {
            self.new_count.fetch_add(1, Ordering::SeqCst);
        }

        fn on_enter(&self, _id: &span::Id, _ctx: Context<'_, CountingSubscriber>) {
            self.enter_count.fetch_add(1, Ordering::SeqCst);
        }

        fn on_exit(&self, _id: &span::Id, _ctx: Context<'_, CountingSubscriber>) {
            self.exit_count.fetch_add(1, Ordering::SeqCst);
        }

        fn on_close(&self, _id: Id, _ctx: Context<'_, CountingSubscriber>) {
            self.close_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    // Setup subscriber and layer.

    let l_new_count = Arc::new(AtomicUsize::new(0));
    let l_enter_count = Arc::new(AtomicUsize::new(0));
    let l_exit_count = Arc::new(AtomicUsize::new(0));
    let l_close_count = Arc::new(AtomicUsize::new(0));
    let layer = CountingLayer {
        new_count: l_new_count.clone(),
        enter_count: l_enter_count.clone(),
        exit_count: l_exit_count.clone(),
        close_count: l_close_count.clone(),
    };

    let s_new_count = Arc::new(AtomicUsize::new(0));
    let s_clone_count = Arc::new(AtomicUsize::new(0));
    let s_enter_count = Arc::new(AtomicUsize::new(0));
    let s_exit_count = Arc::new(AtomicUsize::new(0));
    let s_close_count = Arc::new(AtomicUsize::new(0));

    let subscriber = CountingSubscriber {
        inner: tracing_subscriber::registry(),
        new_count: s_new_count.clone(),
        clone_count: s_clone_count.clone(),
        enter_count: s_enter_count.clone(),
        exit_count: s_exit_count.clone(),
        close_count: s_close_count.clone(),
    };
    let subscriber = Arc::new(subscriber.with(layer));

    // Create a span using the subscriber
    let span = tracing::subscriber::with_default(subscriber.clone(), move || {
        tracing::span!(tracing::Level::INFO, "span")
    });

    // Enter the span in a thread which doesn't have a direct relationship to the subscriber.
    std::thread::spawn(move || {
        let _ = span.entered();
    })
    .join()
    .unwrap();

    // layer should have seen exactly one new span & close
    // should be one enter / exit cycle

    let layer_new_count = l_new_count.load(Ordering::SeqCst);
    let layer_enter_count = l_enter_count.load(Ordering::SeqCst);
    let layer_exit_count = l_exit_count.load(Ordering::SeqCst);
    let layer_close_count = l_close_count.load(Ordering::SeqCst);

    assert_eq!(layer_new_count, 1);
    assert_eq!(layer_enter_count, 1);
    assert_eq!(layer_exit_count, 1);
    assert_eq!(layer_close_count, 1);

    // subscriber should have seen one new span
    // new + any clones should equal number of closes
    // enter and exit should match layer counts

    let sub_new_count = s_new_count.load(Ordering::SeqCst);
    let sub_clone_count = s_clone_count.load(Ordering::SeqCst);
    let sub_enter_count = s_enter_count.load(Ordering::SeqCst);
    let sub_exit_count = s_exit_count.load(Ordering::SeqCst);
    let sub_close_count = s_close_count.load(Ordering::SeqCst);

    assert_eq!(sub_new_count, 1);
    assert_eq!(sub_new_count + sub_clone_count, sub_close_count);

    assert_eq!(sub_enter_count, layer_enter_count);
    assert_eq!(sub_exit_count, layer_exit_count);
}
