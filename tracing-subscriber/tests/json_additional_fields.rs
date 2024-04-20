#![cfg(all(feature = "fmt", feature = "json"))]
//! This test is mostly the same as one in
//! `tracing_subscriber::fmt::format::json`, but having an integration test also
//! checks that all the necessary APIs are accessible from outside the crate.
use std::{
    io,
    sync::{Arc, Mutex},
};

use tracing::{collect::with_default, span, Collect};
use tracing_subscriber::{
    fmt::{format::AdditionalFmtSpanFields, CollectorBuilder},
    registry::LookupSpan,
    subscribe::{self, CollectExt},
    Subscribe,
};

#[derive(Clone)]
struct MockWriter(Arc<Mutex<Vec<u8>>>);

impl io::Write for MockWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

struct InjectingSubscriber;
impl<C: Collect + for<'lookup> LookupSpan<'lookup>> Subscribe<C> for InjectingSubscriber {
    fn on_new_span(
        &self,
        _attrs: &span::Attributes<'_>,
        id: &span::Id,
        ctx: subscribe::Context<'_, C>,
    ) {
        let mut additional = AdditionalFmtSpanFields::default();
        additional.insert("additional".to_owned(), "value".to_owned());
        let span_ref = ctx.span(id).unwrap();
        let mut extensions = span_ref.extensions_mut();
        extensions.insert(additional);
    }
}

#[test]
fn json_additional_fields() {
    let expected =
    "{\"level\":\"INFO\",\"span\":{\"answer\":42,\"name\":\"json_span\",\"number\":3,\"additional\":\"value\"},\"spans\":[{\"answer\":42,\"name\":\"json_span\",\"number\":3,\"additional\":\"value\"}],\"fields\":{\"message\":\"some json test\"}}\n";
    let writer = MockWriter(Arc::new(Mutex::new(Vec::new())));
    let make_writer = {
        let writer = writer.clone();
        move || writer.clone()
    };

    let collector = CollectorBuilder::default()
        .json()
        .without_time()
        .flatten_event(false)
        .with_target(false)
        .with_current_span(true)
        .with_span_list(true)
        .with_additional_span_fields(true)
        .with_writer(make_writer.clone())
        .finish()
        .with(InjectingSubscriber);

    with_default(collector, || {
        let span = tracing::span!(tracing::Level::INFO, "json_span", answer = 42, number = 3);
        let _guard = span.enter();
        tracing::info!("some json test");
    });

    let buf = writer.0.lock().unwrap();
    let actual = std::str::from_utf8(&buf[..]).unwrap();
    assert_eq!(
        serde_json::from_str::<std::collections::HashMap<&str, serde_json::Value>>(expected)
            .unwrap(),
        serde_json::from_str(actual).unwrap()
    );
}
