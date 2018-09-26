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
//! log records (such as the `LogSubscriber` in the `tokio_trace` crate), as
//! doing so will result in the event recursing between the subscriber and the
//! logger forever (or, in real life, probably overflowing the call stack)
//!
//! If the logging of trace events generated from log records produced by the
//! `log` crate is desired, either the `log` crate should not be used to
//! implement this logging, or an additional layer of filtering will be
//! required to avoid infinitely converting between `Event` and `log::Record`.
extern crate tokio_trace;
extern crate log;

use std::{io, time::Instant};
use tokio_trace::{Subscriber, Event};

/// Format a log record as a trace event in the current span.
pub fn format_trace(record: &log::Record) -> io::Result<()> {
    let parent = tokio_trace::Span::current();
    let meta: tokio_trace::Meta = record.into();
    let event = Event {
        timestamp: Instant::now(),
        parent,
        follows_from: &[],
        meta: &meta,
        field_values: &[],
        message: record.args().clone()
    };
    tokio_trace::Dispatcher::current().observe_event(&event);
    Ok(())
}

/// A simple "logger" that converts all log records into `tokio_trace` `Event`s,
/// with an optional level filter.
#[derive(Debug)]
pub struct SimpleTraceLogger {
    filter: log::LevelFilter,
}

impl SimpleTraceLogger {
    pub fn with_filter(filter: log::LevelFilter) -> Self {
       Self {
           filter,
       }
    }
}

impl Default for SimpleTraceLogger {
    fn default() -> Self {
        Self::with_filter(log::LevelFilter::Info)
    }
}

impl log::Log for SimpleTraceLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.filter
    }

    fn log(&self, record: &log::Record) {
        format_trace(record).unwrap();
    }

    fn flush(&self) {}
}
