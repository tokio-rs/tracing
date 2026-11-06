#![cfg(all(feature = "fmt", feature = "json"))]

use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};

#[derive(Clone, Default)]
struct Buffer(Arc<Mutex<Vec<u8>>>);

impl Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn assert_root_event_has_no_span(display_current_span: bool, display_span_list: bool) {
    let buffer = Buffer::default();
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .without_time()
        .with_current_span(display_current_span)
        .with_span_list(display_span_list)
        .with_writer(move || writer.clone())
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let outer = tracing::info_span!("outer", request_id = 123);
        let _outer = outer.enter();
        let inner = tracing::info_span!("inner", operation = "read");
        let _inner = inner.enter();

        tracing::info!(parent: None, "root event");
        tracing::info!("contextual event");
    });

    let output = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
    let events: Vec<serde_json::Value> = output
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(events.len(), 2);

    assert_eq!(events[0]["fields"]["message"], "root event");
    assert!(events[0].get("span").is_none(), "{}", events[0]);
    assert!(events[0].get("spans").is_none(), "{}", events[0]);

    assert_eq!(events[1]["fields"]["message"], "contextual event");
    if display_current_span {
        assert_eq!(events[1]["span"]["name"], "inner");
        assert_eq!(events[1]["span"]["operation"], "read");
    } else {
        assert!(events[1].get("span").is_none());
    }
    if display_span_list {
        let spans = events[1]["spans"].as_array().unwrap();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0]["name"], "outer");
        assert_eq!(spans[0]["request_id"], 123);
        assert_eq!(spans[1]["name"], "inner");
    } else {
        assert!(events[1].get("spans").is_none());
    }
}

#[test]
fn root_event_omits_current_span_and_span_list() {
    assert_root_event_has_no_span(true, true);
}

#[test]
fn root_event_omits_current_span_when_span_list_disabled() {
    assert_root_event_has_no_span(true, false);
}

#[test]
fn root_event_omits_span_list_when_current_span_disabled() {
    assert_root_event_has_no_span(false, true);
}
