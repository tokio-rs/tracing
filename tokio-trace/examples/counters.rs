#[macro_use]
extern crate tokio_trace;

use tokio_trace::{
    field, span,
    subscriber::{self, Subscriber},
    Id, Level, Meta,
};

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
};

#[derive(Clone)]
struct Counters(Arc<RwLock<HashMap<String, AtomicUsize>>>);

struct CounterSubscriber {
    ids: AtomicUsize,
    counters: Counters,
}

impl Subscriber for CounterSubscriber {
    fn register_callsite(&self, meta: &tokio_trace::Meta) -> subscriber::Interest {
        let mut interest = subscriber::Interest::NEVER;
        for key in meta.fields() {
            if let Some(name) = key.name() {
                if name.contains("count") {
                    self.counters
                        .0
                        .write()
                        .unwrap()
                        .entry(name.to_owned())
                        .or_insert_with(|| AtomicUsize::new(0));
                    interest = subscriber::Interest::ALWAYS;
                }
            }
        }
        interest
    }

    fn new_id(&self, _new_span: span::Attributes) -> Id {
        let id = self.ids.fetch_add(1, Ordering::SeqCst);
        Id::from_u64(id as u64)
    }

    fn add_follows_from(&self, _span: &Id, _follows: Id) -> Result<(), subscriber::FollowsError> {
        // unimplemented
        Ok(())
    }

    fn record_i64(
        &self,
        _id: &Id,
        field: &field::Key,
        value: i64,
    ) -> Result<(), ::subscriber::RecordError> {
        let registry = self.counters.0.read().unwrap();
        if let Some(counter) = field.name().and_then(|name| registry.get(name)) {
            if value > 0 {
                counter.fetch_add(value as usize, Ordering::Release);
            } else {
                counter.fetch_sub(value as usize, Ordering::Release);
            }
        };
        Ok(())
    }

    fn record_u64(
        &self,
        _id: &Id,
        field: &field::Key,
        value: u64,
    ) -> Result<(), ::subscriber::RecordError> {
        let registry = self.counters.0.read().unwrap();
        if let Some(counter) = field.name().and_then(|name| registry.get(name)) {
            counter.fetch_add(value as usize, Ordering::Release);
        };
        Ok(())
    }

    /// Adds a new field to an existing span observed by this `Subscriber`.
    ///
    /// This is expected to return an error under the following conditions:
    /// - The span ID does not correspond to a span which currently exists.
    /// - The span does not have a field with the given name.
    /// - The span has a field with the given name, but the value has already
    ///   been set.
    fn record_fmt(
        &self,
        _id: &Id,
        _field: &field::Key,
        _value: ::std::fmt::Arguments,
    ) -> Result<(), ::subscriber::RecordError> {
        Ok(())
    }

    fn enabled(&self, metadata: &Meta) -> bool {
        if !metadata.is_span() {
            return false;
        }
        metadata
            .fields()
            .any(|f| f.name().map(|name| name.contains("count")).unwrap_or(false))
    }

    fn enter(&self, _span: Id) {}
    fn exit(&self, _span: Id) {}
    fn close(&self, _span: Id) {}
}

impl Counters {
    fn print_counters(&self) {
        for (k, v) in self.0.read().unwrap().iter() {
            println!("{}: {}", k, v.load(Ordering::Acquire));
        }
    }

    fn new() -> (Self, CounterSubscriber) {
        let counters = Counters(Arc::new(RwLock::new(HashMap::new())));
        let subscriber = CounterSubscriber {
            ids: AtomicUsize::new(0),
            counters: counters.clone(),
        };
        (counters, subscriber)
    }
}

fn main() {
    let (counters, subscriber) = Counters::new();

    tokio_trace::Dispatch::new(subscriber).as_default(|| {
        let mut foo: u64 = 2;
        span!("my_great_span", foo_count = &foo).enter(|| {
            foo += 1;
            event!(Level::Info, { yak_shaved = true, yak_count = 1i64 }, "hi from inside my span");
            span!("my other span", foo_count = &foo, baz_count = &5u64).enter(|| {
                event!(Level::Warn, { yak_shaved = false, yak_count = 2i64 }, "failed to shave yak");
            });
        });
    });

    counters.print_counters();
}
