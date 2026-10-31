#![cfg(all(feature = "fmt", feature = "registry", feature = "ansi"))]

use std::io::{self, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::{fmt, reload, util::SubscriberInitExt, Registry};

#[derive(Clone, Debug)]
struct TestWriter {
    output: Arc<Mutex<Vec<u8>>>,
}

impl TestWriter {
    fn new() -> Self {
        Self {
            output: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn output(&self) -> String {
        String::from_utf8_lossy(&self.output.lock().unwrap()).into_owned()
    }
}

impl Write for TestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.output.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for TestWriter {
    type Writer = TestWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn compact(writer: TestWriter) -> Box<dyn Layer<Registry> + Send + Sync> {
    fmt::layer()
        .compact()
        .with_writer(writer)
        .with_ansi(false)
        .without_time()
        .boxed()
}

fn pretty(writer: TestWriter) -> Box<dyn Layer<Registry> + Send + Sync> {
    fmt::layer()
        .pretty()
        .with_writer(writer)
        .with_ansi(false)
        .without_time()
        .boxed()
}

#[test]
fn reload_compact_to_pretty_preserves_existing_span_metadata() {
    let writer = TestWriter::new();
    let result = catch_unwind(AssertUnwindSafe(|| {
        let (layer, handle) = reload::Layer::new(compact(writer.clone()));
        let subscriber = tracing_subscriber::registry().with(layer);
        let _guard = subscriber.set_default();

        let span = tracing::info_span!("compact_span", compact_field = "compact_value");
        let _entered = span.enter();
        handle.reload(pretty(writer.clone())).unwrap();
        tracing::info!(event_field = "event_value", "reloaded event");

        let new_span = tracing::info_span!("pretty_span", pretty_field = "pretty_value");
        let _new_entered = new_span.enter();
        tracing::info!("pretty event");
    }));
    let output = writer.output();
    println!("captured output: {output:?}");

    assert!(result.is_ok(), "reloading Compact to Pretty panicked");
    assert!(output.contains("reloaded event"));
    assert!(output.contains("compact_span"));
    assert!(output.contains("event_field"));
    assert!(output.contains("event_value"));
    assert!(output.contains("pretty_span"));
    assert!(output.contains("pretty_field"));
    assert!(output.contains("pretty_value"));
    assert!(output.contains("pretty event"));
}

#[test]
fn pretty_preserves_fields_for_new_spans() {
    let writer = TestWriter::new();
    let subscriber = tracing_subscriber::registry().with(pretty(writer.clone()));
    let _guard = subscriber.set_default();

    let span = tracing::info_span!("pretty_span", pretty_field = "pretty_value");
    let _entered = span.enter();
    tracing::info!("pretty event");

    let output = writer.output();
    assert!(output.contains("pretty_span"));
    assert!(output.contains("pretty_field"));
    assert!(output.contains("pretty_value"));
    assert!(output.contains("pretty event"));
}
