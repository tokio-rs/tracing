//! Adapters for connecting unstructured log records from the `log` crate into
//! the `tracing` ecosystem.
//! 
//! ## Convert log records to tracing `Event`s
//! 
//! To make logs seen as tracing events, set up `LogTracer` as logger by calling
//! its [`init`] or [`init_with_filter`] methods.
//! 
//! ```rust
//! # use std::error::Error;
//! use tracing_log::LogTracer;
//! use log;
//! 
//! # fn main() -> Result<(), Box<Error>> {
//! LogTracer::init()?;
//! 
//! // will be available for Subscribers as a tracing Event
//! log::trace!("an example trace log");
//! # Ok(())
//! # }
//! ```
//! 
//! This conversion does not convert unstructured data in log records (such as
//! values passed as format arguments to the `log!` macro) to structured
//! `tracing` fields. However, it *does* attach these new events to to the
//! span that was currently executing when the record was logged. This is the
//! primary use-case for this library: making it possible to locate the log
//! records emitted by dependencies which use `log` within the context of a
//! trace.
//! 
//! ## Convert tracing `Event`s to logs
//! 
//! This conversion can be done with [`TraceLogger`].
//! 
//! ## Caution: Mixing both conversions
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
//! 
//! [`init`]: struct.LogTracer.html#method.init
//! [`init_with_filter`]: struct.LogTracer.html#method.init_with_filter
//! [`TraceLogger`]: struct.TraceLogger.html
extern crate log;
extern crate tracing_core;
extern crate tracing_subscriber;

use lazy_static::lazy_static;

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

    let (cs, keys) = match record.level() {
        log::Level::Trace => *TRACE_CS,
        log::Level::Debug => *DEBUG_CS,
        log::Level::Info => *INFO_CS,
        log::Level::Warn => *WARN_CS,
        log::Level::Error => *ERROR_CS,
    };

    let log_module = record.module_path();
    let log_file = record.file();
    let log_line = record.line();

    let module = log_module.as_ref().map(|s| s as &dyn field::Value);
    let file = log_file.as_ref().map(|s| s as &dyn field::Value);
    let line = log_line.as_ref().map(|s| s as &dyn field::Value);

    let meta = cs.metadata();
    Event::dispatch(
        &meta,
        &meta.fields().value_set(&[
            (&keys.message, Some(record.args() as &dyn field::Value)),
            (&keys.target, Some(&record.target())),
            (&keys.module, module),
            (&keys.file, file),
            (&keys.line, line),
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

struct Fields {
    message: field::Field,
    target: field::Field,
    module: field::Field,
    file: field::Field,
    line: field::Field,
}

static FIELD_NAMES: &'static [&'static str] = &[
    "message",
    "log.target",
    "log.module_path",
    "log.file",
    "log.line",
];

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
            field::FieldSet::new(FIELD_NAMES, identify_callsite!(&Callsite)),
            Kind::EVENT,
        );

        impl callsite::Callsite for Callsite {
            fn set_interest(&self, _: subscriber::Interest) {}
            fn metadata(&self) -> &'static Metadata<'static> {
                &META
            }
        }

        lazy_static! {
            static ref FIELDS: Fields = {
                let message = META.fields().field("message").unwrap();
                let target = META.fields().field("log.target").unwrap();
                let module = META.fields().field("log.module_path").unwrap();
                let file = META.fields().field("log.file").unwrap();
                let line = META.fields().field("log.line").unwrap();
                Fields {
                    message,
                    target,
                    module,
                    file,
                    line,
                }
            };
        }
        (&Callsite, &FIELDS)
    }};
}

lazy_static! {
    static ref TRACE_CS: (&'static dyn Callsite, &'static Fields) =
        log_cs!(tracing_core::Level::TRACE);
    static ref DEBUG_CS: (&'static dyn Callsite, &'static Fields) =
        log_cs!(tracing_core::Level::DEBUG);
    static ref INFO_CS: (&'static dyn Callsite, &'static Fields) =
        log_cs!(tracing_core::Level::INFO);
    static ref WARN_CS: (&'static dyn Callsite, &'static Fields) =
        log_cs!(tracing_core::Level::WARN);
    static ref ERROR_CS: (&'static dyn Callsite, &'static Fields) =
        log_cs!(tracing_core::Level::ERROR);
}

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
            log::Level::Trace => identify_callsite!(TRACE_CS.0),
            log::Level::Debug => identify_callsite!(DEBUG_CS.0),
            log::Level::Info => identify_callsite!(INFO_CS.0),
            log::Level::Warn => identify_callsite!(WARN_CS.0),
            log::Level::Error => identify_callsite!(ERROR_CS.0),
        };
        Metadata::new(
            "log record",
            self.target(),
            self.level().as_trace(),
            self.file(),
            self.line(),
            self.module_path(),
            field::FieldSet::new(FIELD_NAMES, cs_id),
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

/// A simple "logger" that converts all log records into `tracing` `Event`s.
/// 
/// Can be initialized with:
/// 
/// * [`init`] if you want to convert all logs and do the filtering in a subscriber
/// * [`init_with_filter`] if you know in advance a log level you want to filter
///
/// [`init`]: ../fn.init.html
/// [`init_with_filter`]: ../fn.init_with_filter.html
#[derive(Debug)]
pub struct LogTracer {
    _p: (),
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

// Static logger for `LogTrace`
static LOGGER: LogTracer = LogTracer { _p: () };

impl LogTracer {
    /// Creates a new `LogTracer` that can then be used as logger for the `log` crate.
    /// 
    /// It is generally simpler to use the [`init`] or [`init_with_filter`] methods
    /// that will create the `LogTracer` and set it as global logger.
    /// 
    /// Logger setup without the initialization methods can be done with:
    ///
    /// ```rust
    /// # use std::error::Error;
    /// use tracing_log::LogTracer;
    /// use log;
    /// 
    /// # fn main() -> Result<(), Box<Error>> {
    /// let logger = LogTracer::new();
    /// log::set_boxed_logger(Box::new(logger))?;
    /// log::set_max_level(log::LevelFilter::Trace);
    ///
    /// // will be available for Subscribers as a tracing Event
    /// log::trace!("an example trace log");
    /// # Ok(())
    /// # }
    /// ```
    /// 
    /// [`init`]: #method.init
    /// [`init_with_filter`]: .#method.init_with_filter
    pub fn new() -> Self {
        Self { _p: () }
    }

    /// Sets up `LogTracer` as global logger for the `log` crate,
    /// with the given level as max level filter.
    ///
    /// Setting a global logger can only be done once.
    pub fn init_with_filter(level: log::LevelFilter) -> Result<(), log::SetLoggerError> {
        log::set_logger(&LOGGER)?;
        log::set_max_level(level);
        Ok(())
    }

    /// Sets up `LogTracer` as global logger for the `log` crate.
    ///
    /// Setting a global logger can only be done once.
    /// 
    /// ```rust
    /// # use std::error::Error;
    /// use tracing_log::LogTracer;
    /// use log;
    /// 
    /// # fn main() -> Result<(), Box<Error>> {
    /// LogTracer::init()?;
    /// 
    /// // will be available for Subscribers as a tracing Event
    /// log::trace!("an example trace log");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// This will forward all logs to `tracing` and let the subscribers
    /// do the filtering. If you know in advance you want to filter some log levels,
    /// use [`init_with_filter`] instead.
    ///
    /// [`init_with_filter`]: #method.init_with_filter
    pub fn init() -> Result<(), log::SetLoggerError> {
        Self::init_with_filter(log::LevelFilter::Trace)
    }
}

impl Default for LogTracer {
    fn default() -> Self {
        Self::new()
    }
}

impl log::Log for LogTracer {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        // Use log::set_max_level to filter LogTracing's input
        true
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
