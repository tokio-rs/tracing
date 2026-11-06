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

fn capture_event(display_current_span: bool, emit: impl FnOnce()) -> serde_json::Value {
    let buffer = Buffer::default();
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .without_time()
        .with_current_span(display_current_span)
        .with_writer(move || writer.clone())
        .finish();

    tracing::subscriber::with_default(subscriber, emit);

    let output = buffer.0.lock().unwrap();
    serde_json::from_slice(&output).unwrap()
}

fn assert_explicit_ancestry(enter_unrelated: bool, display_current_span: bool) {
    let event = capture_event(display_current_span, || {
        let root = tracing::info_span!(parent: None, "root", request_id = 123);
        let child = tracing::info_span!(parent: &root, "child", operation = "read");
        let unrelated = tracing::info_span!(parent: None, "unrelated", request_id = 456);
        let _guard = if enter_unrelated {
            Some(unrelated.enter())
        } else {
            None
        };

        tracing::info!(parent: &child, "explicit event");
    });

    assert_eq!(
        event["spans"],
        serde_json::json!([
            {"name": "root", "request_id": 123},
            {"name": "child", "operation": "read"},
        ])
    );
    if display_current_span {
        assert_eq!(event["span"], event["spans"][1]);
    } else {
        assert!(event.get("span").is_none());
    }
}

#[test]
fn explicit_parent_is_serialized_without_entered_spans() {
    assert_explicit_ancestry(false, true);
}

#[test]
fn explicit_parent_overrides_unrelated_entered_span() {
    assert_explicit_ancestry(true, true);
}

#[test]
fn explicit_parent_is_used_when_current_span_is_hidden() {
    assert_explicit_ancestry(true, false);
}

#[test]
fn contextual_event_serializes_its_entered_ancestry() {
    let event = capture_event(true, || {
        let root = tracing::info_span!("root", request_id = 123);
        let _root = root.enter();
        let child = tracing::info_span!("child", operation = "read");
        let _child = child.enter();

        tracing::info!("contextual event");
    });

    assert_eq!(
        event["spans"],
        serde_json::json!([
            {"name": "root", "request_id": 123},
            {"name": "child", "operation": "read"},
        ])
    );
    assert_eq!(event["span"], event["spans"][1]);
}
