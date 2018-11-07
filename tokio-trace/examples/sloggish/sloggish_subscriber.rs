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
    Level, SpanData, SpanId,
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
    stack: Mutex<Vec<SpanId>>,
    spans: Mutex<HashMap<SpanId, SpanData>>,
    ids: AtomicUsize,
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

impl SloggishSubscriber {
    pub fn new(indent_amount: usize) -> Self {
        Self {
            indent_amount,
            stderr: io::stderr(),
            stack: Mutex::new(vec![]),
            spans: Mutex::new(HashMap::new()),
            ids: AtomicUsize::new(0),
        }
    }

    fn print_kvs<'a, I, T>(&self, writer: &mut impl Write, kvs: I, leading: &str) -> io::Result<()>
    where
        I: IntoIterator<Item = (&'a str, T)>,
        T: fmt::Debug,
    {
        let mut kvs = kvs.into_iter();
        if let Some((k, v)) = kvs.next() {
            write!(
                writer,
                "{}{}: {:?}",
                leading,
                Style::new().bold().paint(k),
                v
            )?;
        }
        for (k, v) in kvs {
            write!(writer, ", {}: {:?}", Style::new().bold().paint(k), v)?;
        }
        Ok(())
    }

    fn print_meta(&self, writer: &mut impl Write, meta: &tokio_trace::Meta) -> io::Result<()> {
        write!(
            writer,
            "{level} {target} ",
            level = ColorLevel(meta.level),
            target = meta.target,
        )
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

    fn new_span(&self, span: tokio_trace::SpanData) -> tokio_trace::span::Id {
        let next = self.ids.fetch_add(1, Ordering::SeqCst) as u64;
        let id = tokio_trace::span::Id::from_u64(next);
        self.spans.lock().unwrap().insert(id.clone(), span);
        id
    }

    fn add_value(
        &self,
        span: &tokio_trace::SpanId,
        name: &'static str,
        value: &dyn tokio_trace::IntoValue,
    ) -> Result<(), subscriber::AddValueError> {
        let mut spans = self.spans.lock().expect("mutex poisoned!");
        let span = spans
            .get_mut(span)
            .ok_or(subscriber::AddValueError::NoSpan)?;
        span.add_value(name, value)
    }

    fn add_follows_from(
        &self,
        _span: &tokio_trace::SpanId,
        _follows: tokio_trace::SpanId,
    ) -> Result<(), subscriber::FollowsError> {
        // unimplemented
        Ok(())
    }

    #[inline]
    fn observe_event<'a>(&self, event: &'a tokio_trace::Event<'a>) {
        let mut stderr = self.stderr.lock();

        let stack = self.stack.lock().unwrap();
        if let Some(idx) = stack
            .iter()
            .position(|id| event.parent.as_ref().map(|p| p == id).unwrap_or(false))
        {
            self.print_indent(&mut stderr, idx + 1).unwrap();
        }
        write!(
            &mut stderr,
            "{} ",
            humantime::format_rfc3339_seconds(SystemTime::now())
        ).unwrap();
        self.print_meta(&mut stderr, event.meta).unwrap();
        write!(
            &mut stderr,
            "{}",
            Style::new().bold().paint(format!("{}", event.message))
        ).unwrap();
        self.print_kvs(&mut stderr, event.fields(), ", ").unwrap();
        write!(&mut stderr, "\n").unwrap();
    }

    #[inline]
    fn enter(&self, span: tokio_trace::span::Id) {
        let mut stderr = self.stderr.lock();
        let mut stack = self.stack.lock().unwrap();
        let spans = self.spans.lock().unwrap();
        let data = spans.get(&span);
        let parent = data.and_then(SpanData::parent);
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
                self.print_kvs(&mut stderr, data.fields(), "").unwrap();
            }
            write!(&mut stderr, "\n").unwrap();
        }
    }

    #[inline]
    fn exit(&self, _span: tokio_trace::span::Id) {}

    #[inline]
    fn close(&self, _span: tokio_trace::span::Id) {
        // TODO: it's *probably* safe to remove the span from the cache
        // now...but that doesn't really matter for this example.
    }
}
