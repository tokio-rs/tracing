extern crate tokio_trace_core;

#[cfg(feature = "ansi")]
extern crate ansi_term;

use tokio_trace_core::{
    field,
    Event,
    Span as Id,
    Metadata,
};

use std::{
    fmt,
    io,
    cell::RefCell,
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        RwLock
    },
};

pub mod default;

#[derive(Debug)]
pub struct FmtSubscriber<S=(), E=fn(Context, &mut io::Write, &Event) -> io::Result<()>> {
    fmt_span: S,
    fmt_event: E,
    spans: RwLock<HashMap<Id, SpanData>>,
    next_id: AtomicUsize,
}

pub struct Context<'a> {
    lock: &'a RwLock<HashMap<Id, SpanData>>,
}

#[derive(Debug)]
struct SpanData {
    name: &'static str,
    fields: String,
    ref_count: AtomicUsize,
}

thread_local! {
    static CONTEXT: RefCell<Vec<Id>> = RefCell::new(vec![]);
}

fn curr_id() -> Option<Id> {
    CONTEXT.try_with(|current| current.borrow().last().cloned()).ok()?
}

impl<S, E> FmtSubscriber<S, E> {

    fn span_name(&self, id: &Id) -> Option<&'static str> {
        self.spans.read().ok()?
            .get(id).map(|span| span.name)
    }

    fn enter(&self, id: &Id) {
        let _ = CONTEXT.try_with(|current| {
            current.borrow_mut().push(id.clone());
        });
    }

    fn exit(&self, id: &Id) {
        if let Ok(popped) = CONTEXT.try_with(|current| {
            current.borrow_mut().pop()
        }) {
            debug_assert!(popped.as_ref() == Some(id));
        }
    }
}

impl FmtSubscriber {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for FmtSubscriber {
    fn default() -> Self {
        FmtSubscriber {
            fmt_span: (),
            fmt_event: default::fmt_event,
            spans: RwLock::new(HashMap::default()),
            next_id: AtomicUsize::new(0),
        }
    }
}

impl<S, E> tokio_trace_core::Subscriber for FmtSubscriber<S, E>
where
    E: Fn(Context, &mut io::Write, &Event) -> io::Result<()>,
{
   fn enabled(&self, metadata: &Metadata) -> bool {
       // FIXME: filtering
       true
   }

    fn new_span(&self, metadata: &Metadata, values: &field::ValueSet) -> Id {
        let id = Id::from_u64(self.next_id.fetch_add(1, Ordering::Relaxed) as u64);
        let mut data = SpanData::new(metadata.name());
        values.record(&mut data);
        self.spans.write().expect("rwlock poisoned!")
            .insert(id.clone(), data);
        id
    }

    fn record(&self, span: &Id, values: &field::ValueSet) {
        let mut spans = self.spans.write().expect("rwlock poisoned!");
        if let Some(span) = spans.get_mut(span) {
            values.record(span);
        }
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {}

    fn event(&self, event: &Event) {
        // TODO: better io generation!
        let stdout = io::stdout();
        let mut io = stdout.lock();
        let spans = self.spans.read().expect("rwlock poisoned!");
        if let Err(e) = (self.fmt_event)(Context::new(&self.spans), &mut io, event) {
            eprintln!("error formatting event: {}", e);
        }
    }

    fn enter(&self, span: &Id) {
        // TODO: add on_enter hook
        self.enter(span);
    }

    fn exit(&self, span: &Id)  {
        // TODO: add on_exit hook
        self.exit(span);
    }
}


impl<'a> Context<'a> {
    pub fn fmt_spans<F>(&self, mut f: F) -> fmt::Result
    where
        F: FnMut(&str) -> fmt::Result
    {
        CONTEXT.try_with(|current| {
            let lock = self.lock.read().map_err(|_| fmt::Error)?;
            let stack = current.borrow();
            let spans = stack.iter().filter_map(|id| {
                lock.get(id)
            });
            for span in spans {
                (f)(span.name.as_ref())?;
            }
            Ok(())
        }).map_err(|_| fmt::Error)?
    }

    pub fn fmt_fields<F>(&self, mut f: F) -> fmt::Result
    where
        F: FnMut(&str) -> fmt::Result
    {
        curr_id().map(|id| {
            if let Some(span) = self.lock.read().map_err(|_| fmt::Error)?
                .get(&id)
            {
                (f)(span.fields.as_ref())
            } else {
                Ok(())
            }
        }).unwrap_or(Ok(()))

    }

    fn new(lock: &'a RwLock<HashMap<Id, SpanData>>) -> Self {
        Self {
            lock,
        }
    }
}

// ===== impl SpanData =====

impl SpanData {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            fields: String::new(),
            ref_count: AtomicUsize::new(1),
        }
    }

    #[inline]
    fn clone_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::Release);
    }

    #[inline]
    fn drop_ref(&self) -> bool {
        self.ref_count.fetch_sub(1, Ordering::AcqRel) == 1
    }
}

impl field::Record for SpanData {
    #[inline]
    fn record_str(&mut self, field: &field::Field, value: &str) {
        use std::fmt::Write;
        if field.name() == "message" {
            let _ = write!(self.fields, " {}", value);
        } else {
            self.record_debug(field, &value)
        }
    }

    #[inline]
    fn record_debug(&mut self, field: &field::Field, value: &fmt::Debug) {
        use std::fmt::Write;
        let _ = write!(self.fields, " {}={:?}", field, value);
    }
}
