use crate::AsLog;
use std::{
    collections::HashMap,
    fmt::{self, Write},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};
use tracing_core::{
    field,
    span::{self, Id},
    Event, Metadata, Subscriber,
};

/// A `tracing_core::Subscriber` implementation that logs all recorded
/// trace events.
pub struct TraceLogger {
    settings: Builder,
    spans: Mutex<HashMap<Id, SpanLineBuilder>>,
    current: tracing_subscriber::CurrentSpanPerThread,
    next_id: AtomicUsize,
}

pub struct Builder {
    log_span_closes: bool,
    log_enters: bool,
    log_exits: bool,
    log_ids: bool,
    parent_fields: bool,
    log_parent: bool,
}

// ===== impl TraceLogger =====

impl TraceLogger {
    pub fn new() -> Self {
        Self::builder().finish()
    }

    pub fn builder() -> Builder {
        Default::default()
    }

    fn from_builder(settings: Builder) -> Self {
        Self {
            settings,
            ..Default::default()
        }
    }

    fn next_id(&self) -> Id {
        Id::from_u64(self.next_id.fetch_add(1, Ordering::SeqCst) as u64)
    }
}

// ===== impl Builder =====

impl Builder {
    pub fn with_span_closes(self, log_span_closes: bool) -> Self {
        Self {
            log_span_closes,
            ..self
        }
    }

    pub fn with_parent_fields(self, parent_fields: bool) -> Self {
        Self {
            parent_fields,
            ..self
        }
    }

    pub fn with_span_entry(self, log_enters: bool) -> Self {
        Self { log_enters, ..self }
    }

    pub fn with_span_exits(self, log_exits: bool) -> Self {
        Self { log_exits, ..self }
    }

    pub fn with_ids(self, log_ids: bool) -> Self {
        Self { log_ids, ..self }
    }

    pub fn with_parent_names(self, log_parent: bool) -> Self {
        Self { log_parent, ..self }
    }

    pub fn finish(self) -> TraceLogger {
        TraceLogger::from_builder(self)
    }
}

impl Default for Builder {
    fn default() -> Self {
        Builder {
            log_span_closes: false,
            parent_fields: true,
            log_exits: false,
            log_ids: false,
            log_parent: true,
            log_enters: false,
        }
    }
}

impl Default for TraceLogger {
    fn default() -> Self {
        TraceLogger {
            settings: Default::default(),
            spans: Default::default(),
            current: Default::default(),
            next_id: AtomicUsize::new(1),
        }
    }
}

struct SpanLineBuilder {
    parent: Option<Id>,
    ref_count: usize,
    fields: String,
    file: Option<String>,
    line: Option<u32>,
    module_path: Option<String>,
    target: String,
    level: log::Level,
    name: &'static str,
}

impl SpanLineBuilder {
    fn new(parent: Option<Id>, meta: &Metadata, fields: String) -> Self {
        Self {
            parent,
            ref_count: 1,
            fields,
            file: meta.file().map(String::from),
            line: meta.line(),
            module_path: meta.module_path().map(String::from),
            target: String::from(meta.target()),
            level: meta.level().as_log(),
            name: meta.name(),
        }
    }

    fn log_meta(&self) -> log::Metadata {
        log::MetadataBuilder::new()
            .level(self.level)
            .target(self.target.as_ref())
            .build()
    }

    fn finish(self) {
        let log_meta = self.log_meta();
        let logger = log::logger();
        if logger.enabled(&log_meta) {
            logger.log(
                &log::Record::builder()
                    .metadata(log_meta)
                    .target(self.target.as_ref())
                    .module_path(self.module_path.as_ref().map(String::as_ref))
                    .file(self.file.as_ref().map(String::as_ref))
                    .line(self.line)
                    .args(format_args!("close {}; {}", self.name, self.fields))
                    .build(),
            );
        }
    }
}

impl field::Visit for SpanLineBuilder {
    fn record_debug(&mut self, field: &field::Field, value: &dyn fmt::Debug) {
        write!(self.fields, " {}={:?};", field.name(), value)
            .expect("write to string should never fail")
    }
}

impl Subscriber for TraceLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        log::logger().enabled(&metadata.as_log())
    }

    fn new_span(&self, attrs: &span::Attributes) -> Id {
        let id = self.next_id();
        let mut spans = self.spans.lock().unwrap();
        let mut fields = String::new();
        let parent = self.current.id();
        if self.settings.parent_fields {
            let mut next_parent = parent.as_ref();
            while let Some(ref parent) = next_parent.and_then(|p| spans.get(&p)) {
                write!(&mut fields, "{}", parent.fields).expect("write to string cannot fail");
                next_parent = parent.parent.as_ref();
            }
        }
        let mut span = SpanLineBuilder::new(parent, attrs.metadata(), fields);
        attrs.record(&mut span);
        spans.insert(id.clone(), span);
        id
    }

    fn record(&self, span: &Id, values: &span::Record) {
        let mut spans = self.spans.lock().unwrap();
        if let Some(span) = spans.get_mut(span) {
            values.record(span);
        }
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        // TODO: this should eventually track the relationship?
        log::logger().log(
            &log::Record::builder()
                .level(log::Level::Trace)
                .args(format_args!("span {:?} follows_from={:?};", span, follows))
                .build(),
        );
    }

    fn enter(&self, id: &Id) {
        self.current.enter(id.clone());
        let spans = self.spans.lock().unwrap();
        if self.settings.log_enters {
            if let Some(span) = spans.get(id) {
                let log_meta = span.log_meta();
                let logger = log::logger();
                if logger.enabled(&log_meta) {
                    let current_id = self.current.id();
                    let current_fields = current_id
                        .as_ref()
                        .and_then(|id| spans.get(&id))
                        .map(|span| span.fields.as_ref())
                        .unwrap_or("");
                    if self.settings.log_ids {
                        logger.log(
                            &log::Record::builder()
                                .metadata(log_meta)
                                .target(span.target.as_ref())
                                .module_path(span.module_path.as_ref().map(String::as_ref))
                                .file(span.file.as_ref().map(String::as_ref))
                                .line(span.line)
                                .args(format_args!(
                                    "enter {}; in={:?}; {}",
                                    span.name, current_id, current_fields
                                ))
                                .build(),
                        );
                    } else {
                        logger.log(
                            &log::Record::builder()
                                .metadata(log_meta)
                                .target(span.target.as_ref())
                                .module_path(span.module_path.as_ref().map(String::as_ref))
                                .file(span.file.as_ref().map(String::as_ref))
                                .line(span.line)
                                .args(format_args!("enter {}; {}", span.name, current_fields))
                                .build(),
                        );
                    }
                }
            }
        }
    }

    fn exit(&self, id: &Id) {
        self.current.exit();
        if self.settings.log_exits {
            let spans = self.spans.lock().unwrap();
            if let Some(span) = spans.get(id) {
                let log_meta = span.log_meta();
                let logger = log::logger();
                if logger.enabled(&log_meta) {
                    logger.log(
                        &log::Record::builder()
                            .metadata(log_meta)
                            .target(span.target.as_ref())
                            .module_path(span.module_path.as_ref().map(String::as_ref))
                            .file(span.file.as_ref().map(String::as_ref))
                            .line(span.line)
                            .args(format_args!("exit {}", span.name))
                            .build(),
                    );
                }
            }
        }
    }

    fn event(&self, event: &Event) {
        let meta = event.metadata();
        let log_meta = meta.as_log();
        let logger = log::logger();
        if logger.enabled(&log_meta) {
            let spans = self.spans.lock().unwrap();
            let current = self.current.id().and_then(|id| spans.get(&id));
            let (current_fields, parent) = current
                .map(|span| {
                    let fields = span.fields.as_ref();
                    let parent = if self.settings.log_parent {
                        Some(span.name)
                    } else {
                        None
                    };
                    (fields, parent)
                })
                .unwrap_or(("", None));
            logger.log(
                &log::Record::builder()
                    .metadata(log_meta)
                    .target(meta.target())
                    .module_path(meta.module_path().as_ref().cloned())
                    .file(meta.file().as_ref().cloned())
                    .line(meta.line())
                    .args(format_args!(
                        "{}{}{}{}",
                        parent.unwrap_or(""),
                        if parent.is_some() { ": " } else { "" },
                        LogEvent(event),
                        current_fields,
                    ))
                    .build(),
            );
        }
    }

    fn clone_span(&self, id: &Id) -> Id {
        let mut spans = self.spans.lock().unwrap();
        if let Some(span) = spans.get_mut(id) {
            span.ref_count += 1;
        }
        id.clone()
    }

    fn try_close(&self, id: Id) -> bool {
        let mut spans = self.spans.lock().unwrap();
        if spans.contains_key(&id) {
            if spans.get(&id).unwrap().ref_count == 1 {
                let span = spans.remove(&id).unwrap();
                if self.settings.log_span_closes {
                    span.finish();
                }
                return true;
            } else {
                spans.get_mut(&id).unwrap().ref_count -= 1;
            }
        }
        false
    }
}

struct LogEvent<'a>(&'a Event<'a>);

impl<'a> fmt::Display for LogEvent<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut has_logged = false;
        let mut format_fields = |field: &field::Field, value: &dyn fmt::Debug| {
            let name = field.name();
            let leading = if has_logged { " " } else { "" };
            // TODO: handle fmt error?
            let _ = if name == "message" {
                write!(f, "{}{:?};", leading, value)
            } else {
                write!(f, "{}{}={:?};", leading, name, value)
            };
            has_logged = true;
        };

        self.0.record(&mut format_fields);
        Ok(())
    }
}
