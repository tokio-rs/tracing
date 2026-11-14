#![cfg(all(feature = "fmt", feature = "json"))]

use std::{
    error::Error,
    fmt,
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

#[derive(Debug)]
struct TestError {
    message: &'static str,
    source: Option<Box<TestError>>,
}

impl TestError {
    fn new(message: &'static str) -> Self {
        Self {
            message,
            source: None,
        }
    }

    fn with_source(message: &'static str, source: Self) -> Self {
        Self {
            message,
            source: Some(Box::new(source)),
        }
    }
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}

impl Error for TestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref().map(|source| source as &dyn Error)
    }
}

fn capture_update(previous: TestError, replacement: TestError) -> serde_json::Value {
    let buffer = Buffer::default();
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .without_time()
        .with_writer(move || writer.clone())
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!(
            "operation",
            error = &previous as &dyn Error,
            request_id = 123,
            user.sources = "unchanged",
        );
        span.record("error", &replacement as &dyn Error);
        let _guard = span.enter();
        tracing::info!("updated error");
    });

    let output = buffer.0.lock().unwrap();
    let event: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(event["span"]["name"], "operation");
    assert_eq!(event["span"]["request_id"], 123);
    assert_eq!(event["span"]["user.sources"], "unchanged");
    assert_eq!(event["span"], event["spans"][0]);
    event["span"].clone()
}

#[test]
fn replacing_error_with_no_source_removes_previous_sources() {
    let span = capture_update(
        TestError::with_source("old error", TestError::new("old cause")),
        TestError::new("replacement error"),
    );

    assert_eq!(span["error"], "replacement error");
    assert!(span.get("error.sources").is_none(), "{}", span);
}

#[test]
fn replacing_error_with_a_source_adds_sources() {
    let span = capture_update(
        TestError::new("old error"),
        TestError::with_source("replacement error", TestError::new("new cause")),
    );

    assert_eq!(span["error"], "replacement error");
    assert_eq!(span["error.sources"], serde_json::json!(["new cause"]));
}

#[test]
fn replacing_error_replaces_entire_source_chain() {
    let span = capture_update(
        TestError::with_source(
            "old error",
            TestError::with_source("old cause", TestError::new("old root cause")),
        ),
        TestError::with_source("replacement error", TestError::new("new cause")),
    );

    assert_eq!(span["error"], "replacement error");
    assert_eq!(span["error.sources"], serde_json::json!(["new cause"]));
}
