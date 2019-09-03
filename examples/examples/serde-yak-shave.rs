use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use tracing::{debug, error, info, span, trace, warn};
use tracing_core::{
    event::Event,
    metadata::{Level, Metadata},
    span::{Attributes, Id, Record},
    subscriber::Subscriber,
};
use tracing_serde::AsSerde;

use serde_json::json;

#[path = "fmt/yak_shave.rs"]
mod yak_shave;

pub struct JsonSubscriber {
    next_id: AtomicUsize, // you need to assign span IDs, so you need a counter
}

impl Subscriber for JsonSubscriber {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        let json = json!({
        "enabled": {
            "metadata": metadata.as_serde(),
        }});
        println!("{}", json);
        true
    }

    fn new_span(&self, attrs: &Attributes<'_>) -> Id {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let id = Id::from_u64(id as u64);
        let json = json!({
        "new_span": {
            "attributes": attrs.as_serde(),
            "id": id.as_serde(),
        }});
        println!("{}", json);
        id
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        let json = json!({
        "record": {
            "span": span.as_serde(),
            "values": values.as_serde(),
        }});
        println!("{}", json);
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        let json = json!({
        "record_follows_from": {
            "span": span.as_serde(),
            "follows": follows.as_serde(),
        }});
        println!("{}", json);
    }

    fn event(&self, event: &Event<'_>) {
        let json = json!({
            "event": event.as_serde(),
        });
        println!("{}", json);
    }

    fn enter(&self, span: &Id) {
        let json = json!({
            "enter": span.as_serde(),
        });
        println!("{}", json);
    }

    fn exit(&self, span: &Id) {
        let json = json!({
            "exit": span.as_serde(),
        });
        println!("{}", json);
    }
}

fn main() {
    let subscriber = JsonSubscriber {
        next_id: AtomicUsize::new(1),
    };

    tracing::subscriber::with_default(subscriber, || {
        let number_of_yaks = 3;
        debug!("preparing to shave {} yaks", number_of_yaks);

        let number_shaved = yak_shave::shave_all(number_of_yaks);

        debug!(
            message = "yak shaving completed.",
            all_yaks_shaved = number_shaved == number_of_yaks,
        );
    });
}
