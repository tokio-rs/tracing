#[macro_use]
extern crate tokio_trace;

use tokio_trace::{
    span,
    subscriber::{self, Subscriber},
    Event, IntoValue, Level, Meta, Value,
};

use std::{
    any::Any,
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
    spans: RwLock<HashMap<span::Id, span::Data>>,
}

impl Subscriber for CounterSubscriber {
    fn new_span(&self, new_span: span::Data) -> span::Id {
        let id = self.ids.fetch_add(1, Ordering::SeqCst);
        let id = span::Id::from_u64(id as u64);
        let mut registry = self.counters.0.write().unwrap();
        for name in new_span.meta().field_names {
            if name.contains("count") {
                let _ = registry.entry(name).or_insert_with(|| AtomicUsize::new(0));
            }
        }
        self.spans.write().unwrap().insert(id.clone(), new_span);
        id
    }

    fn add_value(
        &self,
        span: &span::Id,
        name: &'static str,
        value: &dyn IntoValue,
    ) -> Result<(), subscriber::AddValueError> {
        self.spans
            .write()
            .unwrap()
            .get_mut(span)
            .ok_or(subscriber::AddValueError::NoSpan)?
            .add_value(name, value)
    }

    fn enabled(&self, metadata: &Meta) -> bool {
        metadata.is_span() && metadata
            .field_names
            .iter()
            .any(|name| name.contains("count"))
    }

    fn should_invalidate_filter(&self, _metadata: &Meta) -> bool {
        false
    }

    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {}

    fn enter(&self, span: span::Id, state: span::State) {}

    fn exit(&self, span: span::Id, state: span::State) {
        if state != span::State::Done {
            return;
        }
        if let Some(span) = self.spans.read().unwrap().get(&span) {
            let registry = self.counters.0.read().unwrap();
            for (counter, value) in span.fields().into_iter().filter_map(|(k, v)| {
                if !k.contains("count") {
                    return None;
                }
                let v = v.downcast_ref::<usize>()?;
                let c = registry.get(k)?;
                Some((c, v))
            }) {
                counter.fetch_add(*value, Ordering::Release);
            }
        }
    }
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
            spans: RwLock::new(HashMap::new()),
        };
        (counters, subscriber)
    }
}

fn main() {
    let (counters, subscriber) = Counters::new();

    tokio_trace::Dispatch::to(subscriber).with(|| {
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
