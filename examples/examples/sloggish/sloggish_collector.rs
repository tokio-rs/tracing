use ansi_term::{Color, Style};
use tracing::{
    field::{Field, Visit},
    Collect, Id, Level, Metadata,
};
use tracing_core::span::Current;

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
    time::SystemTime,
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

pub struct SloggishCollector {
    // TODO: this can probably be unified with the "stack" that's used for
    // printing?
    current: CurrentSpanPerThread,
    indent_amount: usize,
    stderr: io::Stderr,
    stack: Mutex<Vec<Id>>,
    spans: Mutex<HashMap<Id, Span>>,
    ids: AtomicUsize,
}

struct Span {
    parent: Option<Id>,
    kvs: Vec<(&'static str, String)>,
    metadata: &'static Metadata<'static>,
}

struct Event<'a> {
    stderr: io::StderrLock<'a>,
    comma: bool,
}

struct ColorLevel<'a>(&'a Level);

impl<'a> fmt::Display for ColorLevel<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self.0 {
            Level::TRACE => Color::Purple.paint("TRACE"),
            Level::DEBUG => Color::Blue.paint("DEBUG"),
            Level::INFO => Color::Green.paint("INFO "),
            Level::WARN => Color::Yellow.paint("WARN "),
            Level::ERROR => Color::Red.paint("ERROR"),
        }
        .fmt(f)
    }
}

impl Span {
    fn new(parent: Option<Id>, attrs: &tracing::span::Attributes<'_>) -> Self {
        let mut span = Self {
            parent,
            kvs: Vec::new(),
            metadata: attrs.metadata(),
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
            &mut self.stderr,
            "{comma} ",
            comma = if self.comma { "," } else { "" },
        )
        .unwrap();
        let name = field.name();
        if name == "message" {
            write!(
                &mut self.stderr,
                "{}",
                // Have to alloc here due to `ansi_term`'s API...
                Style::new().bold().paint(format!("{:?}", value))
            )
            .unwrap();
        } else {
            write!(
                &mut self.stderr,
                "{}: {:?}",
                Style::new().bold().paint(name),
                value
            )
            .unwrap();
        }
        self.comma = true;
    }
}

impl SloggishCollector {
    pub fn new(indent_amount: usize) -> Self {
        Self {
            current: CurrentSpanPerThread::new(),
            indent_amount,
            stderr: io::stderr(),
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

impl Collect for SloggishCollector {
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
        let mut stderr = self.stderr.lock();
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
            self.print_indent(&mut stderr, indent).unwrap();
            stack.push(span_id.clone());
            if let Some(data) = data {
                self.print_kvs(&mut stderr, data.kvs.iter().map(|(k, v)| (k, v)), "")
                    .unwrap();
            }
            writeln!(&mut stderr).unwrap();
        }
    }

    fn event(&self, event: &tracing::Event<'_>) {
        let mut stderr = self.stderr.lock();
        let indent = self.stack.lock().unwrap().len();
        self.print_indent(&mut stderr, indent).unwrap();
        write!(
            &mut stderr,
            "{timestamp} {level} {target}",
            timestamp = humantime::format_rfc3339_seconds(SystemTime::now()),
            level = ColorLevel(event.metadata().level()),
            target = &event.metadata().target(),
        )
        .unwrap();
        let mut visitor = Event {
            stderr,
            comma: false,
        };
        event.record(&mut visitor);
        writeln!(&mut visitor.stderr).unwrap();
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

    fn current_span(&self) -> Current {
        if let Some(id) = self.current.id() {
            let metadata = self
                .spans
                .lock()
                .unwrap()
                .get(&id)
                .unwrap_or_else(|| panic!("no metadata stored for span with ID {:?}", id))
                .metadata;
            return Current::new(id, metadata);
        }
        Current::none()
    }
}
