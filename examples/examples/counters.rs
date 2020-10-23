#![deny(rust_2018_idioms)]

use tracing::{
    collect::{self, Collect},
    field::{Field, Visit},
    info, span, warn, Event, Id, Level, Metadata,
};

use std::{
    collections::HashMap,
    fmt,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock, RwLockReadGuard,
    },
};

#[derive(Clone)]
struct Counters(Arc<RwLock<HashMap<String, AtomicUsize>>>);

struct CounterCollector {
    ids: AtomicUsize,
    counters: Counters,
}

struct Count<'a> {
    counters: RwLockReadGuard<'a, HashMap<String, AtomicUsize>>,
}

impl<'a> Visit for Count<'a> {
    fn record_i64(&mut self, field: &Field, value: i64) {
        if let Some(counter) = self.counters.get(field.name()) {
            if value > 0 {
                counter.fetch_add(value as usize, Ordering::Release);
            } else {
                counter.fetch_sub(-value as usize, Ordering::Release);
            }
        };
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if let Some(counter) = self.counters.get(field.name()) {
            counter.fetch_add(value as usize, Ordering::Release);
        };
    }

    fn record_bool(&mut self, _: &Field, _: bool) {}
    fn record_str(&mut self, _: &Field, _: &str) {}
    fn record_debug(&mut self, _: &Field, _: &dyn fmt::Debug) {}
}

impl CounterCollector {
    fn visitor(&self) -> Count<'_> {
        Count {
            counters: self.counters.0.read().unwrap(),
        }
    }
}

impl Collect for CounterCollector {
    fn register_callsite(&self, meta: &Metadata<'_>) -> collect::Interest {
        let mut interest = collect::Interest::never();
        for key in meta.fields() {
            let name = key.name();
            if name.contains("count") {
                self.counters
                    .0
                    .write()
                    .unwrap()
                    .entry(name.to_owned())
                    .or_insert_with(|| AtomicUsize::new(0));
                interest = collect::Interest::always();
            }
        }
        interest
    }

    fn new_span(&self, new_span: &span::Attributes<'_>) -> Id {
        new_span.record(&mut self.visitor());
        let id = self.ids.fetch_add(1, Ordering::SeqCst);
        Id::from_u64(id as u64)
    }

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {
        // unimplemented
    }

    fn record(&self, _: &Id, values: &span::Record<'_>) {
        values.record(&mut self.visitor())
    }

    fn event(&self, event: &Event<'_, '_>) {
        event.record(&mut self.visitor())
    }

    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.fields().iter().any(|f| f.name().contains("count"))
    }

    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

impl Counters {
    fn print_counters(&self) {
        for (k, v) in self.0.read().unwrap().iter() {
            println!("{}: {}", k, v.load(Ordering::Acquire));
        }
    }

    fn new() -> (Self, CounterCollector) {
        let counters = Counters(Arc::new(RwLock::new(HashMap::new())));
        let collector = CounterCollector {
            ids: AtomicUsize::new(1),
            counters: counters.clone(),
        };
        (counters, collector)
    }
}

fn main() {
    let (counters, collector) = Counters::new();

    tracing::collect::set_global_default(collector).unwrap();

    let mut foo: u64 = 2;
    span!(Level::TRACE, "my_great_span", foo_count = &foo).in_scope(|| {
        foo += 1;
        info!(yak_shaved = true, yak_count = 1, "hi from inside my span");
        span!(
            Level::TRACE,
            "my other span",
            foo_count = &foo,
            baz_count = 5
        )
        .in_scope(|| {
            warn!(yak_shaved = false, yak_count = -1, "failed to shave yak");
        });
    });

    counters.print_counters();
}
