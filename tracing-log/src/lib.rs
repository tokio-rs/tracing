//! Adapters for connecting unstructured log records from the `log` crate into
//! the `tracing` ecosystem.
//!
//! This conversion does not convert unstructured data in log records (such as
//! values passed as format arguments to the `log!` macro) to structured
//! `tracing` fields. However, it *does* attach these new events to to the
//! span that was currently executing when the record was logged. This is the
//! primary use-case for this library: making it possible to locate the log
//! records emitted by dependencies which use `log` within the context of a
//! trace.
//!
//! Note that logger implementations that convert log records to trace events
//! should not be used with `Subscriber`s that convert trace events _back_ into
//! log records (such as the `TraceLogger`), as doing so will result in the
//! event recursing between the subscriber and the logger forever (or, in real
//! life, probably overflowing the call stack).
//!
//! If the logging of trace events generated from log records produced by the
//! `log` crate is desired, either the `log` crate should not be used to
//! implement this logging, or an additional layer of filtering will be
//! required to avoid infinitely converting between `Event` and `log::Record`.
extern crate log;
extern crate tracing_core;
extern crate tracing_subscriber;

use std::{
    collections::HashMap,
    fmt::{self, Write},
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

use tracing_core::{
    callsite::{self, Callsite},
    dispatcher, field, identify_callsite,
    metadata::Kind,
    span::{self, Id},
    subscriber::{self, Subscriber},
    Event, Metadata,
};

/// Format a log record as a trace event in the current span.
pub fn format_trace(record: &log::Record) -> io::Result<()> {
    let filter_meta = record.as_trace();
    if !dispatcher::get_default(|dispatch| dispatch.enabled(&filter_meta)) {
        return Ok(());
    };

    let cs = match record.level() {
        log::Level::Trace => TRACE_CS,
        log::Level::Debug => DEBUG_CS,
        log::Level::Info => INFO_CS,
        log::Level::Warn => WARN_CS,
        log::Level::Error => ERROR_CS,
    };
    let meta = cs.metadata();
    let fields = meta.fields();
    let key = fields
        .field(&"message")
        .expect("log record fields must have a message");
    let target_key = fields
        .field(&"log.target")
        .expect("log record fields must have a target");
    let module_key = fields
        .field(&"log.module_path")
        .expect("log record fields must have a module_path");
    let file_key = fields
        .field(&"log.file")
        .expect("log record fields must have a file");
    let line_key = fields
        .field(&"log.line")
        .expect("log record fields must have a line");
    Event::dispatch(
        &meta,
        &fields.value_set(&[
            (&key, Some(record.args() as &dyn field::Value)),
            (&target_key, Some(&record.target())),
            (&module_key, record.module_path().as_ref()
                              .map(|s| s as &dyn field::Value)),
            (&file_key, record.file().as_ref()
                              .map(|s| s as &dyn field::Value)),
            (&line_key, record.line().as_ref()
                              .map(|s| s as &dyn field::Value)),
        ]),
    );
    Ok(())
}

pub trait AsLog {
    type Log;
    fn as_log(&self) -> Self::Log;
}

pub trait AsTrace {
    type Trace;
    fn as_trace(&self) -> Self::Trace;
}

macro_rules! log_cs {
    ($level:expr) => {{
        struct Callsite;
        static META: Metadata = Metadata::new(
            "log event",
            "log",
            $level,
            None,
            None,
            None,
            field::FieldSet::new(
                &[
                    "message",
                    "log.target",
                    "log.module_path",
                    "log.file",
                    "log.line",
                ],
                identify_callsite!(&Callsite),
            ),
            Kind::EVENT,
        );

        impl callsite::Callsite for Callsite {
            fn set_interest(&self, _: subscriber::Interest) {}
            fn metadata(&self) -> &'static Metadata<'static> {
                &META
            }
        }
        &Callsite
    }};
}

static TRACE_CS: &'static dyn Callsite = log_cs!(tracing_core::Level::TRACE);
static DEBUG_CS: &'static dyn Callsite = log_cs!(tracing_core::Level::DEBUG);
static INFO_CS: &'static dyn Callsite = log_cs!(tracing_core::Level::INFO);
static WARN_CS: &'static dyn Callsite = log_cs!(tracing_core::Level::WARN);
static ERROR_CS: &'static dyn Callsite = log_cs!(tracing_core::Level::ERROR);

impl<'a> AsLog for Metadata<'a> {
    type Log = log::Metadata<'a>;
    fn as_log(&self) -> Self::Log {
        log::Metadata::builder()
            .level(self.level().as_log())
            .target(self.target())
            .build()
    }
}
impl<'a> AsTrace for log::Record<'a> {
    type Trace = Metadata<'a>;
    fn as_trace(&self) -> Self::Trace {
        let cs_id = match self.level() {
            log::Level::Trace => identify_callsite!(TRACE_CS),
            log::Level::Debug => identify_callsite!(DEBUG_CS),
            log::Level::Info => identify_callsite!(INFO_CS),
            log::Level::Warn => identify_callsite!(WARN_CS),
            log::Level::Error => identify_callsite!(ERROR_CS),
        };
        Metadata::new(
            "log record",
            self.target(),
            self.level().as_trace(),
            self.module_path(),
            self.line(),
            self.file(),
            field::FieldSet::new(
                &[
                    "message",
                    "log.target",
                    "log.module_path",
                    "log.file",
                    "log.line",
                ],
                cs_id,
            ),
            Kind::EVENT,
        )
    }
}

impl AsLog for tracing_core::Level {
    type Log = log::Level;
    fn as_log(&self) -> log::Level {
        match *self {
            tracing_core::Level::ERROR => log::Level::Error,
            tracing_core::Level::WARN => log::Level::Warn,
            tracing_core::Level::INFO => log::Level::Info,
            tracing_core::Level::DEBUG => log::Level::Debug,
            tracing_core::Level::TRACE => log::Level::Trace,
        }
    }
}

impl AsTrace for log::Level {
    type Trace = tracing_core::Level;
    fn as_trace(&self) -> tracing_core::Level {
        match self {
            log::Level::Error => tracing_core::Level::ERROR,
            log::Level::Warn => tracing_core::Level::WARN,
            log::Level::Info => tracing_core::Level::INFO,
            log::Level::Debug => tracing_core::Level::DEBUG,
            log::Level::Trace => tracing_core::Level::TRACE,
        }
    }
}

/// A simple "logger" that converts all log records into `tracing` `Event`s,
/// with an optional level filter.
#[derive(Debug)]
pub struct LogTracer {
    filter: log::LevelFilter,
}

/// A `tracing_subscriber::Observe` implementation that logs all recorded
/// trace events.
pub struct TraceLogger {
    settings: TraceLoggerBuilder,
    spans: Mutex<HashMap<Id, SpanLineBuilder>>,
    current: tracing_subscriber::CurrentSpanPerThread,
    next_id: AtomicUsize,
}

pub struct TraceLoggerBuilder {
    log_span_closes: bool,
    log_enters: bool,
    log_exits: bool,
    log_ids: bool,
    parent_fields: bool,
    log_parent: bool,
}

// ===== impl LogTracer =====

impl LogTracer {
    pub fn with_filter(filter: log::LevelFilter) -> Self {
        Self { filter }
    }
}

impl Default for LogTracer {
    fn default() -> Self {
        Self::with_filter(log::LevelFilter::Info)
    }
}

impl log::Log for LogTracer {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.filter
    }

    fn log(&self, record: &log::Record) {
        let enabled = dispatcher::get_default(|dispatch| {
            // TODO: can we cache this for each log record, so we can get
            // similar to the callsite cache?
            dispatch.enabled(&record.as_trace())
        });

        if enabled {
            // TODO: if the record is enabled, we'll get the current dispatcher
            // twice --- once to check if enabled, and again to dispatch the event.
            // If we could construct events without dispatching them, we could
            // re-use the dispatcher reference...
            format_trace(record).unwrap();
        }
    }

    fn flush(&self) {}
}

// ===== impl TraceLogger =====

impl TraceLogger {
    pub fn new() -> Self {
        Self::builder().finish()
    }

    pub fn builder() -> TraceLoggerBuilder {
        Default::default()
    }

    fn from_builder(settings: TraceLoggerBuilder) -> Self {
        Self {
            settings,
            ..Default::default()
        }
    }

    fn next_id(&self) -> Id {
        Id::from_u64(self.next_id.fetch_add(1, Ordering::SeqCst) as u64)
    }
}

// ===== impl TraceLoggerBuilder =====

impl TraceLoggerBuilder {
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

impl Default for TraceLoggerBuilder {
    fn default() -> Self {
        TraceLoggerBuilder {
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

    fn drop_span(&self, id: Id) {
        let mut spans = self.spans.lock().unwrap();
        if spans.contains_key(&id) {
            if spans.get(&id).unwrap().ref_count == 1 {
                let span = spans.remove(&id).unwrap();
                if self.settings.log_span_closes {
                    span.finish();
                }
            } else {
                spans.get_mut(&id).unwrap().ref_count -= 1;
            }
            return;
        }
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

// impl tracing_subscriber::Observe for TraceLogger {
//     fn observe_event<'a>(&self, event: &'a Event<'a>) {
//         <Self as Subscriber>::observe_event(&self, event)
//     }

//     fn enter(&self, span: &SpanRef) {
//         if let Some(data) = span.data {
//             let meta = data.metadata();
//             let log_meta = meta.as_log();
//             let logger = log::logger();
//             if logger.enabled(&log_meta) {
//                 logger.log(
//                     &log::Record::builder()
//                         .metadata(log_meta)
//                         .module_path(meta.module_path)
//                         .file(meta.file)
//                         .line(meta.line)
//                         .args(format_args!(
//                             "enter: {}; span={:?}; parent={:?}; {:?}",
//                             meta.name.unwrap_or(""),
//                             span.id,
//                             data.parent(),
//                             LogFields(span),
//                         )).build(),
//                 );
//             }
//         } else {
//             <Self as Subscriber>::enter(&self, span.id.clone())
//         }
//     }

//     fn exit(&self, span: &SpanRef) {
//         if let Some(data) = span.data {
//             let meta = data.metadata();
//             let log_meta = meta.as_log();
//             let logger = log::logger();
//             if logger.enabled(&log_meta) {
//                 logger.log(
//                     &log::Record::builder()
//                         .metadata(log_meta)
//                         .module_path(meta.module_path)
//                         .file(meta.file)
//                         .line(meta.line)
//                         .args(format_args!(
//                             "exit: {}; span={:?}; parent={:?};",
//                             meta.name.unwrap_or(""),
//                             span.id,
//                             data.parent(),
//                         )).build(),
//                 );
//             }
//         } else {
//             <Self as Subscriber>::exit(&self, span.id.clone())
//         }
//     }

//     fn close(&self, span: &SpanRef) {
//         if let Some(data) = span.data {
//             let meta = data.metadata();
//             let log_meta = meta.as_log();
//             let logger = log::logger();
//             if logger.enabled(&log_meta) {
//                 logger.log(
//                     &log::Record::builder()
//                         .metadata(log_meta)
//                         .module_path(meta.module_path)
//                         .file(meta.file)
//                         .line(meta.line)
//                         .args(format_args!(
//                             "close: {}; span={:?}; parent={:?};",
//                             meta.name.unwrap_or(""),
//                             span.id,
//                             data.parent(),
//                         )).build(),
//                 );
//             }
//         } else {
//             <Self as Subscriber>::close(&self, span.id.clone())
//         }
//     }

//     fn filter(&self) -> &tracing_subscriber::Filter {
//         self
//     }
// }
