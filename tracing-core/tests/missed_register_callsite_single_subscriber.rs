use std::{
    ptr,
    sync::atomic::{AtomicPtr, Ordering},
    thread,
    time::Duration,
};

use tracing_core::{
    callsite::{Callsite as _, DefaultCallsite},
    dispatcher,
    field::{FieldSet, Value},
    span, Event, Kind, Level, Metadata, Subscriber,
};

struct TestSubscriber {
    sleep: Duration,
    callsite: AtomicPtr<Metadata<'static>>,
}

impl TestSubscriber {
    fn new(sleep_micros: u64) -> Self {
        Self {
            sleep: Duration::from_micros(sleep_micros),
            callsite: AtomicPtr::new(ptr::null_mut()),
        }
    }
}

impl Subscriber for TestSubscriber {
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> tracing_core::Interest {
        println!(
            "{thread:?}: TestSubscriber::register_callsite: start",
            thread = std::thread::current().name()
        );
        if !self.sleep.is_zero() {
            thread::sleep(self.sleep);
        }

        self.callsite
            .store(metadata as *const _ as *mut _, Ordering::SeqCst);

        println!(
            "{thread:?}: TestSubscriber::register_callsite: end",
            thread = std::thread::current().name()
        );
        tracing_core::Interest::always()
    }

    fn event(&self, event: &tracing_core::Event<'_>) {
        println!(
            "{thread:?}: TestSubscriber::register_callsite: start",
            thread = std::thread::current().name()
        );
        let stored_callsite = self.callsite.load(Ordering::SeqCst);
        let event_callsite: *mut Metadata<'static> = event.metadata() as *const _ as *mut _;

        // This assert is the actual test.
        assert_eq!(
            stored_callsite, event_callsite,
            "stored callsite: {stored_callsite:#?} does not match event \
            callsite: {event_callsite:#?}. Was `event` called before \
            `register_callsite`?"
        );
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

fn emit_event() {
    let thread = thread::current();
    static CALLSITE: DefaultCallsite = {
        // The values of the metadata are unimportant
        static META: Metadata<'static> = Metadata::new(
            "event ",
            "module::path",
            Level::INFO,
            None,
            None,
            None,
            FieldSet::new(&["message"], tracing_core::callsite::Identifier(&CALLSITE)),
            Kind::EVENT,
        );
        DefaultCallsite::new(&META)
    };
    let _interest = CALLSITE.interest();

    let meta = CALLSITE.metadata();
    let field = meta.fields().field("message").unwrap();
    let message = format!("event-from-{idx}", idx = thread.name().unwrap_or("unnamed"));
    let values = [(&field, Some(&message as &dyn Value))];
    let value_set = CALLSITE.metadata().fields().value_set(&values);

    Event::dispatch(meta, &value_set);
}

/// Regression test for missing register_callsite call (#2743)
///
/// This test provokes the race condition which causes the only (global) subscriber to not receive
/// a call to `register_callsite` before it receives a call to `event`. This occurs when the
/// (first) call to `register_callsite` takes a long time to complete and the second thread that
/// attempts to register the same callsite finds that some other thread is already registering and
/// leaves `DefaultCallsite::register` before the first registration is complete.
///
/// Because the test depends on the interaction of multiple dispatchers in different threads,
/// it needs to be in a test file by itself.
#[test]
fn event_before_register() {
    let subscriber_register_sleep_micros = 1000;

    let subscriber = TestSubscriber::new(subscriber_register_sleep_micros);
    dispatcher::set_global_default(subscriber.into()).unwrap();

    let jh1 = thread::Builder::new().name("thread-1".into()).spawn(emit_event).unwrap();
    let jh2 = thread::Builder::new().name("thread-2".into()).spawn(emit_event).unwrap();

    jh1.join().expect("failed to join thread");
    jh2.join().expect("failed to join thread");
}
