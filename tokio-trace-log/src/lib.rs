//! Adapters for connecting unstructured log records from the `log` crate into
//! the `tokio_trace` ecosystem.
//!
//! This conversion does not convert unstructured data in log records (such as
//! values passed as format arguments to the `log!` macro) to structured
//! `tokio_trace` fields. However, it *does* attach these new events to to the
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
extern crate tokio_trace;
extern crate tokio_trace_subscriber;

use std::{
    collections::HashMap,
    fmt::{self, Write},
    io,
    sync::{
        atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT},
        Mutex,
    },
};
use tokio_trace::{
    field, span,
    subscriber::{self, Subscriber},
    Id, Meta,
};

/// Format a log record as a trace event in the current span.
pub fn format_trace(record: &log::Record) -> io::Result<()> {
    struct LogCallsite<'a>(tokio_trace::Meta<'a>);
    impl<'a> tokio_trace::Callsite for LogCallsite<'a> {
        fn interest(&self) -> tokio_trace::subscriber::Interest {
            tokio_trace::subscriber::Interest::SOMETIMES
        }

        fn add_interest(&self, _interest: tokio_trace::subscriber::Interest) {
            // Since these callsites can't be registered (they're not known to
            // be valid for the 'static lifetime), we don't need to track
            // interest --- do nothing.
        }

        fn remove_interest(&self) {
            // Again, we don't cache interest for these.
        }

        fn metadata(&self) -> &Meta {
            &self.0
        }
    }
    let callsite = LogCallsite(record.as_trace());
    let k = callsite.0.key_for(&"message").unwrap();
    drop(tokio_trace::Event::new(&callsite, |event| {
        event.message(&k, record.args().clone()).unwrap();
    }));
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

impl<'a> AsLog for Meta<'a> {
    type Log = log::Metadata<'a>;
    fn as_log(&self) -> Self::Log {
        log::Metadata::builder()
            .level(self.level.as_log())
            .target(self.target)
            .build()
    }
}

impl<'a> AsTrace for log::Record<'a> {
    type Trace = Meta<'a>;
    fn as_trace(&self) -> Self::Trace {
        Meta::new_id(
            self.target(),
            self.level().as_trace(),
            self.module_path(),
            self.file(),
            self.line(),
            &["message"],
        )
    }
}

impl AsLog for tokio_trace::Level {
    type Log = log::Level;
    fn as_log(&self) -> log::Level {
        match self {
            tokio_trace::Level::Error => log::Level::Error,
            tokio_trace::Level::Warn => log::Level::Warn,
            tokio_trace::Level::Info => log::Level::Info,
            tokio_trace::Level::Debug => log::Level::Debug,
            tokio_trace::Level::Trace => log::Level::Trace,
        }
    }
}

impl AsTrace for log::Level {
    type Trace = tokio_trace::Level;
    fn as_trace(&self) -> tokio_trace::Level {
        match self {
            log::Level::Error => tokio_trace::Level::Error,
            log::Level::Warn => tokio_trace::Level::Warn,
            log::Level::Info => tokio_trace::Level::Info,
            log::Level::Debug => tokio_trace::Level::Debug,
            log::Level::Trace => tokio_trace::Level::Trace,
        }
    }
}

/// A simple "logger" that converts all log records into `tokio_trace` `Event`s,
/// with an optional level filter.
#[derive(Debug)]
pub struct LogTracer {
    filter: log::LevelFilter,
}

/// A `tokio_trace_subscriber::Observe` implementation that logs all recorded
/// trace events.
pub struct TraceLogger {
    settings: TraceLoggerBuilder,
    in_progress: Mutex<InProgress>,
}

#[derive(Default)]
pub struct TraceLoggerBuilder {
    log_span_closes: bool,
    log_enters: bool,
    log_exits: bool,
    log_ids: bool,
    parent_fields: bool,
}

struct InProgress {
    spans: HashMap<Id, SpanLineBuilder>,
    events: HashMap<Id, EventLineBuilder>,
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
        format_trace(record).unwrap();
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
            in_progress: Mutex::new(InProgress {
                spans: HashMap::new(),
                events: HashMap::new(),
            }),
        }
    }

    fn next_id(&self) -> Id {
        static NEXT_ID: AtomicUsize = ATOMIC_USIZE_INIT;
        Id::from_u64(NEXT_ID.fetch_add(1, Ordering::SeqCst) as u64)
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

    pub fn finish(self) -> TraceLogger {
        TraceLogger::from_builder(self)
    }
}

struct SpanLineBuilder {
    attrs: span::SpanAttributes,
    log_line: String,
    fields: String,
}

impl SpanLineBuilder {
    fn new(
        attrs: span::SpanAttributes,
        fields: String,
        id: Id,
        settings: &TraceLoggerBuilder,
    ) -> Self {
        let mut log_line = String::new();
        let meta = attrs.metadata();
        write!(&mut log_line, "{}", meta.name.unwrap_or("unknown span"),)
            .expect("write to string shouldn't fail");
        if settings.log_ids {
            write!(
                &mut log_line,
                "{}span={:?}; parent={:?};",
                if meta.name.is_some() { " " } else { "" },
                id,
                attrs.parent(),
            ).expect("write to string shouldn't fail");
        }
        Self {
            attrs,
            log_line,
            fields,
        }
    }

    fn record(&mut self, key: &field::Key, val: fmt::Arguments) -> fmt::Result {
        write!(&mut self.fields, "{}=", key.name().unwrap_or("???"))?;
        self.fields.write_fmt(val)?;
        self.fields.write_str("; ")
    }

    fn finish(self) {
        let meta = self.attrs.metadata();
        let log_meta = meta.as_log();
        let logger = log::logger();
        if logger.enabled(&log_meta) {
            logger.log(
                &log::Record::builder()
                    .metadata(log_meta)
                    .target(meta.target)
                    .module_path(meta.module_path)
                    .file(meta.file)
                    .line(meta.line)
                    .args(format_args!("close {}; {}", self.log_line, self.fields))
                    .build(),
            );
        }
    }
}

struct EventLineBuilder {
    level: tokio_trace::Level,
    file: Option<String>,
    line: Option<u32>,
    module_path: Option<String>,
    target: String,
    message: String,
    log_line: String,
}

impl EventLineBuilder {
    fn new(attrs: span::Attributes, id: Id, settings: &TraceLoggerBuilder) -> Self {
        let mut log_line = String::new();
        let meta = attrs.metadata();
        if settings.log_ids {
            write!(
                &mut log_line,
                "event={:?}; parent={:?}; ",
                id,
                attrs.parent(),
            ).expect("write to string shouldn't fail");
        }
        Self {
            level: meta.level,
            file: meta.file.map(|s| s.to_owned()),
            line: meta.line,
            module_path: meta.module_path.map(|s| s.to_owned()),
            target: meta.target.to_owned(),
            message: String::new(),
            log_line,
        }
    }

    fn record(&mut self, key: &field::Key, val: fmt::Arguments) -> fmt::Result {
        if key.name() == Some("message") {
            self.message.write_fmt(val)
        } else {
            write!(&mut self.log_line, "{}=", key.name().unwrap_or("???"))?;
            self.log_line.write_fmt(val)?;
            self.log_line.write_str("; ")
        }
    }

    fn finish(self, current_span: &str) {
        let logger = log::logger();
        let log_meta = log::MetadataBuilder::new()
            .level(self.level.as_log())
            .target(self.target.as_ref())
            .build();
        if logger.enabled(&log_meta) {
            logger.log(
                &log::Record::builder()
                    .metadata(log_meta)
                    .target(self.target.as_ref())
                    .module_path(self.module_path.as_ref().map(String::as_ref))
                    .file(self.file.as_ref().map(String::as_ref))
                    .line(self.line)
                    .args(format_args!(
                        "{}; {}{}",
                        self.message, current_span, self.log_line
                    )).build(),
            );
        }
    }
}

impl Subscriber for TraceLogger {
    fn enabled(&self, metadata: &Meta) -> bool {
        log::logger().enabled(&metadata.as_log())
    }

    fn new_span(&self, new_span: span::SpanAttributes) -> Id {
        let id = self.next_id();
        let mut in_progress = self.in_progress.lock().unwrap();
        let mut fields = String::new();
        if self.settings.parent_fields {
            let mut next_parent = new_span.parent();
            while let Some(ref parent) = next_parent.and_then(|p| in_progress.spans.get(&p)) {
                write!(&mut fields, "{}", parent.fields).expect("write to string cannot fail");
                next_parent = parent.attrs.parent();
            }
        }
        in_progress.spans.insert(
            id.clone(),
            SpanLineBuilder::new(new_span, fields, id.clone(), &self.settings),
        );
        id
    }

    fn new_id(&self, new_span: span::Attributes) -> Id {
        let id = self.next_id();
        self.in_progress.lock().unwrap().events.insert(
            id.clone(),
            EventLineBuilder::new(new_span, id.clone(), &self.settings),
        );
        id
    }

    fn record_fmt(
        &self,
        span: &Id,
        key: &field::Key,
        val: fmt::Arguments,
    ) -> Result<(), subscriber::RecordError> {
        let mut in_progress = self.in_progress.lock().unwrap();
        if let Some(span) = in_progress.spans.get_mut(span) {
            span.record(key, val)?;
            return Ok(());
        }
        if let Some(event) = in_progress.events.get_mut(span) {
            event.record(key, val)?;
            Ok(())
        } else {
            Err(subscriber::RecordError::no_span(span.clone()))
        }
    }

    fn add_follows_from(&self, span: &Id, follows: Id) -> Result<(), subscriber::FollowsError> {
        // TODO: this should eventually track the relationship?
        log::logger().log(
            &log::Record::builder()
                .level(log::Level::Trace)
                .args(format_args!("span {:?} follows_from={:?};", span, follows))
                .build(),
        );
        Ok(())
    }

    fn enter(&self, span: Id) {
        if self.settings.log_enters {
            let logger = log::logger();
            logger.log(
                &log::Record::builder()
                    .level(log::Level::Trace)
                    .args(format_args!("enter: span={:?};", span))
                    .build(),
            );
        }
    }

    fn exit(&self, span: Id) {
        if self.settings.log_exits {
            let logger = log::logger();
            logger.log(
                &log::Record::builder()
                    .level(log::Level::Trace)
                    .args(format_args!("exit: id={:?};", span))
                    .build(),
            );
        }
    }

    fn close(&self, id: Id) {
        let mut in_progress = self.in_progress.lock().unwrap();
        if let Some(span) = in_progress.spans.remove(&id) {
            if self.settings.log_span_closes {
                span.finish();
            }
            return;
        };
        let event = in_progress.events.remove(&id);
        if let Some(event) = event {
            if let Some(id) = Id::current() {
                if let Some(ref span) = in_progress.spans.get(&id) {
                    return event.finish(&span.fields);
                }
            }
            event.finish("")
        }
    }
}

// impl tokio_trace_subscriber::Observe for TraceLogger {
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

//     fn filter(&self) -> &dyn tokio_trace_subscriber::Filter {
//         self
//     }
// }

impl tokio_trace_subscriber::Filter for TraceLogger {
    fn enabled(&self, metadata: &Meta) -> bool {
        <Self as Subscriber>::enabled(&self, metadata)
    }

    fn should_invalidate_filter(&self, _metadata: &Meta) -> bool {
        false
    }
}
