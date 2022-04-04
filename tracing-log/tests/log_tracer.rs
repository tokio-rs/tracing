use std::sync::{Arc, Mutex};
use tracing::collect::with_default;
use tracing_core::span::{Attributes, Record};
use tracing_core::{span, Collect, Event, Level, LevelFilter, Metadata};
use tracing_log::LogTracer;

struct State {
    last_normalized_metadata: Mutex<Option<OwnedMetadata>>,
}

#[derive(PartialEq, Debug)]
struct OwnedMetadata {
    name: String,
    target: String,
    level: Level,
    module_path: Option<String>,
    file: Option<String>,
    line: Option<u32>,
}

impl From<Metadata<'_>> for OwnedMetadata {
    fn from(meta: Metadata<'_>) -> Self {
        Self {
            name: meta.name().to_string(),
            target: meta.target().to_string(),
            level: *meta.level(),
            module_path: meta.module_path().map(String::from),
            file: meta.file().map(String::from),
            line: meta.line(),
        }
    }
}

struct TestSubscriber(Arc<State>);

impl Collect for TestSubscriber {
    fn enabled(&self, meta: &Metadata<'_>) -> bool {
        dbg!(meta);
        true
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(LevelFilter::from_level(Level::INFO))
    }

    fn new_span(&self, _span: &Attributes<'_>) -> span::Id {
        span::Id::from_u64(42)
    }

    fn record(&self, _span: &span::Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {}

    fn event(&self, event: &Event<'_>) {
        dbg!(event);
        *self.0.last_normalized_metadata.lock().unwrap() = Some(event.metadata().into());
    }

    fn enter(&self, _span: &span::Id) {}

    fn exit(&self, _span: &span::Id) {}

    fn current_span(&self) -> span::Current {
        span::Current::unknown()
    }
}

#[test]
fn normalized_metadata() {
    LogTracer::init().unwrap();
    let me = Arc::new(State {
        last_normalized_metadata: Mutex::new(None),
    });
    let state = me.clone();

    with_default(TestSubscriber(me), || {
        log::info!("expected info log");
        log::debug!("unexpected debug log");
        let log = log::Record::builder()
            .args(format_args!("Error!"))
            .level(log::Level::Info)
            .build();
        log::logger().log(&log);
        last(
            &state,
            &OwnedMetadata {
                name: "log event".to_string(),
                target: "".to_string(),
                level: Level::INFO,
                module_path: None,
                file: None,
                line: None,
            },
        );

        let log = log::Record::builder()
            .args(format_args!("Error!"))
            .level(log::Level::Info)
            .target("log_tracer_target")
            .file(Some("server.rs"))
            .line(Some(144))
            .module_path(Some("log_tracer"))
            .build();
        log::logger().log(&log);
        last(
            &state,
            &OwnedMetadata {
                name: "log event".to_string(),
                target: "log_tracer_target".to_string(),
                level: Level::INFO,
                module_path: Some("log_tracer".to_string()),
                file: Some("server.rs".to_string()),
                line: Some(144),
            },
        );

        tracing::info!("test with a tracing info");
        let line = line!() - 1;
        let file = file!();
        last(
            &state,
            &OwnedMetadata {
                name: format!("event {file}:{line}"),
                target: module_path!().to_string(),
                level: Level::INFO,
                module_path: Some(module_path!().to_string()),
                file: Some(file.to_string()),
                line: Some(line),
            },
        );
    })
}

fn last(state: &State, expected: &OwnedMetadata) {
    let lock = state.last_normalized_metadata.lock().unwrap();
    let metadata = &*lock;
    dbg!(&metadata);
    assert_eq!(metadata.as_ref(), Some(expected));
}
