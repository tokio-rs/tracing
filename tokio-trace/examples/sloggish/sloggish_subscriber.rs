//! A simple example demonstrating how one might implement a custom
//! subscriber.
//!
//! This subscriber implements a tree-structured logger similar to
//! the "compact" formatter in [`slog-term`]. The demo mimicks the
//! example output in the screenshot in the [`slog` README].
//!
//! Note that this logger isn't ready for actual production use.
//! Several corners were cut to make the example simple.
//!
//! [`slog-term`]: https://docs.rs/slog-term/2.4.0/slog_term/
//! [`slog` README]: https://github.com/slog-rs/slog#terminal-output-example
extern crate ansi_term;
extern crate humantime;
use self::ansi_term::{Color, Style};
use super::tokio_trace::{
    self,
    subscriber::{self, Subscriber},
    Id, Level, SpanAttributes,
};

use std::{
    collections::HashMap,
    fmt,
    io::{self, Write},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    time::SystemTime,
};

pub struct SloggishSubscriber {
    indent_amount: usize,
    stderr: io::Stderr,
    stack: Mutex<Vec<Id>>,
    spans: Mutex<HashMap<Id, Span>>,
    events: Mutex<HashMap<Id, Event>>,
    ids: AtomicUsize,
}

struct Span {
    attrs: SpanAttributes,
    kvs: Vec<(String, String)>,
}

struct Event {
    id: Id,
    level: tokio_trace::Level,
    target: String,
    message: String,
    kvs: Vec<(String, String)>,
}

struct ColorLevel(Level);

impl fmt::Display for ColorLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Level::Trace => Color::Purple.paint("TRACE"),
            Level::Debug => Color::Blue.paint("DEBUG"),
            Level::Info => Color::Green.paint("INFO"),
            Level::Warn => Color::Yellow.paint("WARN "),
            Level::Error => Color::Red.paint("ERROR"),
        }.fmt(f)
    }
}

impl Span {
    fn new(attrs: SpanAttributes) -> Self {
        Self {
            attrs,
            kvs: Vec::new(),
        }
    }

    fn record(
        &mut self,
        key: &tokio_trace::field::Key,
        value: fmt::Arguments,
    ) -> Result<(), subscriber::RecordError> {
        // value.record(key, &mut tokio_trace::field::DebugRecorder::new(&mut s))?;
        // TODO: shouldn't have to alloc the key...
        let k = key.name().unwrap_or("???").to_owned();
        let v = fmt::format(value);
        self.kvs.push((k, v));
        Ok(())
    }
}

impl Event {
    fn new(attrs: tokio_trace::Attributes, id: Id) -> Self {
        let meta = attrs.metadata();
        Self {
            id,
            target: meta.target.to_owned(),
            level: meta.level,
            message: String::new(),
            kvs: Vec::new(),
        }
    }

    fn record(
        &mut self,
        key: &tokio_trace::field::Key,
        value: fmt::Arguments,
    ) -> Result<(), subscriber::RecordError> {
        if key.name() == Some("message") {
            self.message = fmt::format(value);
            return Ok(());
        }

        // TODO: shouldn't have to alloc the key...
        let k = key.name().unwrap_or("???").to_owned();
        let v = fmt::format(value);
        self.kvs.push((k, v));
        Ok(())
    }
}

impl SloggishSubscriber {
    pub fn new(indent_amount: usize) -> Self {
        Self {
            indent_amount,
            stderr: io::stderr(),
            stack: Mutex::new(vec![]),
            spans: Mutex::new(HashMap::new()),
            events: Mutex::new(HashMap::new()),
            ids: AtomicUsize::new(0),
        }
    }

    fn print_kvs<'a, I, K, V>(
        &self,
        writer: &mut impl Write,
        kvs: I,
        leading: &str,
    ) -> io::Result<()>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str> + 'a,
        V: fmt::Display + 'a,
    {
        let mut kvs = kvs.into_iter();
        if let Some((k, v)) = kvs.next() {
            write!(
                writer,
                "{}{}: {}",
                leading,
                Style::new().bold().paint(k.as_ref()),
                v
            )?;
        }
        for (k, v) in kvs {
            write!(writer, ", {}: {}", Style::new().bold().paint(k.as_ref()), v)?;
        }
        Ok(())
    }

    fn print_indent(&self, writer: &mut impl Write, indent: usize) -> io::Result<()> {
        for _ in 0..(indent * self.indent_amount) {
            write!(writer, " ")?;
        }
        Ok(())
    }
}

impl Subscriber for SloggishSubscriber {
    fn enabled(&self, _metadata: &tokio_trace::Meta) -> bool {
        true
    }

    fn new_id(&self, span: tokio_trace::span::Attributes) -> tokio_trace::Id {
        let next = self.ids.fetch_add(1, Ordering::SeqCst) as u64;
        let id = tokio_trace::Id::from_u64(next);
        self.events
            .lock()
            .unwrap()
            .insert(id.clone(), Event::new(span, id.clone()));
        id
    }

    fn new_span(&self, span: tokio_trace::span::SpanAttributes) -> tokio_trace::Id {
        let next = self.ids.fetch_add(1, Ordering::SeqCst) as u64;
        let id = tokio_trace::Id::from_u64(next);
        self.spans
            .lock()
            .unwrap()
            .insert(id.clone(), Span::new(span));
        id
    }

    fn record_fmt(
        &self,
        span: &tokio_trace::Id,
        name: &tokio_trace::field::Key,
        value: fmt::Arguments,
    ) -> Result<(), subscriber::RecordError> {
        let mut events = self.events.lock().expect("mutex poisoned!");
        if let Some(event) = events.get_mut(span) {
            return event.record(name, value);
        };
        let mut spans = self.spans.lock().expect("mutex poisoned!");
        let span = spans
            .get_mut(span)
            .ok_or_else(|| subscriber::RecordError::no_span(span.clone()))?;
        span.record(name, value)?;
        Ok(())
    }

    fn add_follows_from(
        &self,
        _span: &tokio_trace::Id,
        _follows: tokio_trace::Id,
    ) -> Result<(), subscriber::FollowsError> {
        // unimplemented
        Ok(())
    }

    #[inline]
    fn enter(&self, span: tokio_trace::Id) {
        let mut stderr = self.stderr.lock();
        let mut stack = self.stack.lock().unwrap();
        let spans = self.spans.lock().unwrap();
        let data = spans.get(&span);
        let parent = data.and_then(|span| span.attrs.parent());
        if stack.iter().any(|id| id == &span) {
            // We are already in this span, do nothing.
            return;
        } else {
            let indent = if let Some(idx) = stack
                .iter()
                .position(|id| parent.map(|p| id == p).unwrap_or(false))
            {
                let idx = idx + 1;
                stack.truncate(idx);
                idx
            } else {
                stack.clear();
                0
            };
            self.print_indent(&mut stderr, indent).unwrap();
            stack.push(span);
            if let Some(data) = data {
                self.print_kvs(&mut stderr, data.kvs.iter().map(|(k, v)| (k, v)), "")
                    .unwrap();
            }
            write!(&mut stderr, "\n").unwrap();
        }
    }

    #[inline]
    fn exit(&self, _span: tokio_trace::Id) {}

    #[inline]
    fn close(&self, id: tokio_trace::Id) {
        if let Some(event) = self.events.lock().expect("mutex poisoned").remove(&id) {
            let mut stderr = self.stderr.lock();
            let indent = self.stack.lock().unwrap().len();
            self.print_indent(&mut stderr, indent).unwrap();
            write!(
                &mut stderr,
                "{timestamp} {level} {target} {message}",
                timestamp = humantime::format_rfc3339_seconds(SystemTime::now()),
                level = ColorLevel(event.level),
                target = &event.target,
                message = Style::new().bold().paint(event.message),
            ).unwrap();
            self.print_kvs(
                &mut stderr,
                event.kvs.iter().map(|&(ref k, ref v)| (k, v)),
                ", ",
            ).unwrap();
            write!(&mut stderr, "\n").unwrap();
        }
        // TODO: it's *probably* safe to remove the span from the cache
        // now...but that doesn't really matter for this example.
    }
}
