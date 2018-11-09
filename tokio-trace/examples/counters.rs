#[macro_use]
extern crate tokio_trace;

use tokio_trace::{
    field, span,
    subscriber::{self, Subscriber},
    Event, IntoValue, Level, Meta, Value,
};

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
};

#[derive(Clone)]
struct Counters(Arc<RwLock<HashMap<&'static str, AtomicUsize>>>);

struct CounterSubscriber {
    ids: AtomicUsize,
    counters: Counters,
}

impl Subscriber for CounterSubscriber {
    fn new_span(&self, new_span: span::Data) -> span::Id {
        let id = self.ids.fetch_add(1, Ordering::SeqCst);
        for key in new_span.field_keys() {
            if let Some(name) = key.name() {
                if name.contains("count") {
                    self.counters
                        .0
                        .write()
                        .unwrap()
                        .entry(name)
                        .or_insert_with(|| AtomicUsize::new(0));
                }
            }
        }
        span::Id::from_u64(id as u64)
    }

    fn add_value(
        &self,
        _span: &span::Id,
        key: &field::Key,
        value: &dyn IntoValue,
    ) -> Result<(), subscriber::AddValueError> {
        let registry = self.counters.0.read().unwrap();
        if let Some((counter, value)) = key.name().and_then(|name| {
            let counter = registry.get(name)?;
            let &val = value.into_value().downcast_ref::<usize>()?;
            Some((counter, val))
        }) {
            counter.fetch_add(value, Ordering::Release);
        }
        Ok(())
    }

    fn add_follows_from(
        &self,
        _span: &span::Id,
        _follows: span::Id,
    ) -> Result<(), subscriber::FollowsError> {
        // unimplemented
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

    fn should_invalidate_filter(&self, _metadata: &Meta) -> bool {
        false
    }

    fn observe_event<'a>(&self, _event: &'a Event<'a>) {}

    fn enter(&self, _span: span::Id) {}
    fn exit(&self, _span: span::Id) {}
    fn close(&self, _span: span::Id) {}
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

    tokio_trace::Dispatch::to(subscriber).as_default(|| {
        let mut foo: usize = 2;
        span!("my_great_span", foo_count = &foo).enter(|| {
            foo += 1;
            event!(
                Level::Info,
                { yak_shaved = &true },
                "hi from inside my span"
            );
            span!("my other span", foo_count = &foo, baz_count = &5usize).enter(|| {})
        });
    });

    counters.print_counters();
}
