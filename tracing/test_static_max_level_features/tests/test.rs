#[macro_use]
extern crate tracing;

use std::sync::{Arc, Mutex};
use tracing::span::{Attributes, Record};
use tracing::{span, Event, Id, Level, Metadata, Collector};

struct State {
    last_level: Mutex<Option<Level>>,
}

struct TestCollector(Arc<State>);

impl Collector for TestCollector {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn new_span(&self, _span: &Attributes) -> Id {
        span::Id::from_u64(42)
    }

    fn record(&self, _span: &Id, _values: &Record) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event) {
        *self.0.last_level.lock().unwrap() = Some(event.metadata().level().clone());
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn test_static_max_level_features() {
    let me = Arc::new(State {
        last_level: Mutex::new(None),
    });
    let a = me.clone();
    tracing::collector::with_default(TestCollector(me), || {
        error!("");
        last(&a, Some(Level::ERROR));
        warn!("");
        last(&a, Some(Level::WARN));
        info!("");
        last(&a, Some(Level::INFO));
        debug!("");
        last(&a, Some(Level::DEBUG));
        trace!("");
        last(&a, None);

        span!(Level::ERROR, "");
        last(&a, None);
        span!(Level::WARN, "");
        last(&a, None);
        span!(Level::INFO, "");
        last(&a, None);
        span!(Level::DEBUG, "");
        last(&a, None);
        span!(Level::TRACE, "");
        last(&a, None);
    });
}

fn last(state: &State, expected: Option<Level>) {
    let mut lvl = state.last_level.lock().unwrap();
    assert_eq!(*lvl, expected);
    *lvl = None;
}
