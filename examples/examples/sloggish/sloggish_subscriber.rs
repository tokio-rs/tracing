//! A simple example demonstrating how one might implement a custom
//! subscriber.
//!
//! This subscriber implements a tree-structured logger similar to
//! the "compact" formatter in [`slog-term`]. The demo mimics the
//! example output in the screenshot in the [`slog` README].
//!
//! Note that this logger isn't ready for actual production use.
//! Several corners were cut to make the example simple.
//!
//! [`slog-term`]: https://docs.rs/slog-term/2.4.0/slog_term/
//! [`slog` README]: https://github.com/slog-rs/slog#terminal-output-example
use self::ansi_term::{Color, Style};
use ansi_term;
use chrono::prelude::*;
use tracing::{
    self,
    field::{Field, Visit},
    Id, Level, Subscriber,
};

use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    io::{self, Write},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    thread,
};

/// Tracks the currently executing span on a per-thread basis.
#[derive(Clone)]
pub struct CurrentSpanPerThread {
    current: &'static thread::LocalKey<RefCell<Vec<Id>>>,
}

impl CurrentSpanPerThread {
    pub fn new() -> Self {
        thread_local! {
            static CURRENT: RefCell<Vec<Id>> = RefCell::new(vec![]);
        };
        Self { current: &CURRENT }
    }

    /// Returns the [`Id`](::Id) of the span in which the current thread is
    /// executing, or `None` if it is not inside of a span.
    pub fn id(&self) -> Option<Id> {
        self.current
            .with(|current| current.borrow().last().cloned())
    }

    pub fn enter(&self, span: Id) {
        self.current.with(|current| {
            current.borrow_mut().push(span);
        })
    }

    pub fn exit(&self) {
        self.current.with(|current| {
            let _ = current.borrow_mut().pop();
        })
    }
}

pub struct SloggishSubscriber {
    // TODO: this can probably be unified with the "stack" that's used for
    // printing?
    current: CurrentSpanPerThread,
    indent_amount: usize,
    stdout: io::Stdout,
    stack: Mutex<Vec<Id>>,
    spans: Mutex<HashMap<Id, Span>>,
    ids: AtomicUsize,
}

struct Span {
    parent: Option<Id>,
    name: &'static str,
    kvs: Vec<(&'static str, String)>,
}

struct Event<'a> {
    stdout: io::StdoutLock<'a>,
    comma: bool,
}

struct ColorLevel<'a>(&'a Level);

impl<'a> fmt::Display for ColorLevel<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self.0 {
            Level::TRACE => Color::Purple.bold().paint("TRACE"),
            Level::DEBUG => Color::Blue.bold().paint("DEBUG"),
            Level::INFO => Color::Green.bold().paint(" INFO"),
            Level::WARN => Color::RGB(252, 234, 160).bold().paint(" WARN"), // orange
            Level::ERROR => Color::Red.bold().paint("ERROR"),
        }
        .fmt(f)
    }
}

impl Span {
    fn new(parent: Option<Id>, attrs: &tracing::span::Attributes<'_>) -> Self {
        let mut span = Self {
            parent,
            name: attrs.metadata().name(),
            kvs: Vec::new(),
        };
        attrs.record(&mut span);
        span
    }
}

impl Visit for Span {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.kvs.push((field.name(), format!("{:?}", value)))
    }
}

impl<'a> Visit for Event<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        write!(
            &mut self.stdout,
            "{comma} ",
            comma = if self.comma { "," } else { "" },
        )
        .unwrap();
        let name = field.name();
        if name == "message" {
            write!(
                &mut self.stdout,
                "{}",
                // Have to alloc here due to `ansi_term`'s API...
                Style::new().paint(format!("{:?}", value))
            )
            .unwrap();
            self.comma = true;
        } else {
            write!(
                &mut self.stdout,
                "{}={:?}",
                name,
                // Style::new().bold().fg(Color::Purple).paint(name),
                value
            )
            .unwrap();
            self.comma = true;
        }
    }
}

impl SloggishSubscriber {
    pub fn new(indent_amount: usize) -> Self {
        Self {
            current: CurrentSpanPerThread::new(),
            indent_amount,
            stdout: io::stdout(),
            stack: Mutex::new(vec![]),
            spans: Mutex::new(HashMap::new()),
            ids: AtomicUsize::new(1),
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
                "{}{}={}",
                leading,
                // Style::new().fg(Color::Purple).bold().paint(k.as_ref()),
                k.as_ref(),
                v
            )?;
        }
        for (k, v) in kvs {
            write!(
                writer,
                ", {}={}",
                // Style::new().fg(Color::Purple).bold().paint(k.as_ref()),
                k.as_ref(),
                v
            )?;
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
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &tracing::span::Attributes<'_>) -> tracing::Id {
        let next = self.ids.fetch_add(1, Ordering::SeqCst) as u64;
        let id = tracing::Id::from_u64(next);
        let span = Span::new(self.current.id(), span);
        self.spans.lock().unwrap().insert(id.clone(), span);
        id
    }

    fn record(&self, span: &tracing::Id, values: &tracing::span::Record<'_>) {
        let mut spans = self.spans.lock().expect("mutex poisoned!");
        if let Some(span) = spans.get_mut(span) {
            values.record(span);
        }
    }

    fn record_follows_from(&self, _span: &tracing::Id, _follows: &tracing::Id) {
        // unimplemented
    }

    fn enter(&self, span_id: &tracing::Id) {
        self.current.enter(span_id.clone());
        let mut stdout = self.stdout.lock();
        let mut stack = self.stack.lock().unwrap();
        let spans = self.spans.lock().unwrap();
        let data = spans.get(span_id);
        let parent = data.and_then(|span| span.parent.as_ref());
        if !stack.iter().any(|id| id == span_id) {
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
            self.print_indent(&mut stdout, indent).unwrap();
            stack.push(span_id.clone());
            if let Some(data) = data {
                write!(
                    &mut stdout,
                    "{name}",
                    name = Style::new().fg(Color::Green).bold().paint(data.name)
                )
                .unwrap();
                write!(
                    &mut stdout,
                    "{}",
                    Style::new().fg(Color::Green).paint("{") // Style::new().fg(Color::Green).dimmed().paint("{")
                )
                .unwrap();
                self.print_kvs(&mut stdout, data.kvs.iter().map(|(k, v)| (k, v)), "")
                    .unwrap();
                write!(
                    &mut stdout,
                    "{}",
                    Style::new().fg(Color::Green).bold().paint("}") // Style::new().dimmed().paint("}")
                )
                .unwrap();
            }
            writeln!(&mut stdout).unwrap();
        }
    }

    fn event(&self, event: &tracing::Event<'_>) {
        let mut stdout = self.stdout.lock();
        let indent = self.stack.lock().unwrap().len();
        self.print_indent(&mut stdout, indent).unwrap();
        let now = Local::now();
        write!(
            &mut stdout,
            "{timestamp} {level}",
            timestamp = Style::new()
                .dimmed()
                .paint(now.format("%b %-d, %-I:%M:%S").to_string()),
            level = ColorLevel(event.metadata().level())
        )
        .unwrap();
        let mut visitor = Event {
            stdout,
            comma: false,
        };
        event.record(&mut visitor);
        writeln!(&mut visitor.stdout).unwrap();
    }

    #[inline]
    fn exit(&self, _span: &tracing::Id) {
        // TODO: unify stack with current span
        self.current.exit();
    }

    fn try_close(&self, _id: tracing::Id) -> bool {
        // TODO: GC unneeded spans.
        false
    }
}
