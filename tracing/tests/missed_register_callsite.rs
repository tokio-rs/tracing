use std::{
    ptr,
    sync::atomic::{AtomicPtr, Ordering},
    thread::{self, JoinHandle},
    time::Duration,
};

use tracing::Subscriber;
use tracing_core::{span, Metadata};

struct TestSubscriber {
    creator_thread: String,
    sleep: Duration,
    callsite: AtomicPtr<Metadata<'static>>,
}

impl TestSubscriber {
    fn new(sleep_micros: u64) -> Self {
        let creator_thread = thread::current()
            .name()
            .unwrap_or("<unknown thread>")
            .to_owned();
        Self {
            creator_thread,
            sleep: Duration::from_micros(sleep_micros),
            callsite: AtomicPtr::new(ptr::null_mut()),
        }
    }
}

impl Subscriber for TestSubscriber {
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> tracing_core::Interest {
        if !self.sleep.is_zero() {
            thread::sleep(self.sleep);
        }

        self.callsite
            .store(metadata as *const _ as *mut _, Ordering::SeqCst);
        println!(
            "{creator}: register_callsite: {callsite:#?}",
            creator = self.creator_thread,
            callsite = metadata as *const _,
        );
        tracing_core::Interest::always()
    }

    fn event(&self, event: &tracing_core::Event<'_>) {
        let stored_callsite = self.callsite.load(Ordering::SeqCst);
        let event_callsite: *mut Metadata<'static> = event.metadata() as *const _ as *mut _;

        println!(
            "{creator}: event (with callsite): {event_callsite:#?} (stored callsite: {stored_callsite:#?})",
            creator = self.creator_thread,
        );

        // This assert is the actual test.
        // assert_eq!(
        //     stored_callsite, event_callsite,
        //     "stored callsite: {stored_callsite:#?} does not match event \
        //     callsite: {event_callsite:#?}. Was `event` called before \
        //     `register_callsite`?"
        // );
    }

    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _span: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(0)
    }
    fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {}
    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {}
    fn enter(&self, _span: &tracing_core::span::Id) {}
    fn exit(&self, _span: &tracing_core::span::Id) {}
}

#[test]
fn event_before_register() {
    fn subscriber_thread(idx: usize, sleep_micros: u64) -> JoinHandle<()> {
        thread::Builder::new()
            .name(format!("subscriber-{idx}"))
            .spawn(move || {
                let subscriber = TestSubscriber::new(sleep_micros);
                let _subscriber_guard = tracing::subscriber::set_default(subscriber);

                tracing::info!("event-from-{idx}", idx = idx);
                thread::sleep(Duration::from_millis(100));
            })
            .expect("failed to spawn thread")
    }

    let register_sleep_micros = 50;
    let jh1 = subscriber_thread(1, register_sleep_micros);
    let jh2 = subscriber_thread(2, 0);

    jh1.join().expect("failed to join thread");
    jh2.join().expect("failed to join thread");
}
