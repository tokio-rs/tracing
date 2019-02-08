extern crate tokio_trace_core;

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
pub struct FmtSubscriber<S, E> {
    fmt_span: S,
    fmt_event: E,
    spans: RwLock<HashMap<Id, SpanData>>,
    next_id: AtomicUsize,
}

pub struct Context {
    _p: (),
}

#[derive(Debug)]
struct SpanData {
    name: &'static str,
}

thread_local! {
    static CONTEXT: RefCell<Vec<&'static str>> = RefCell::new(vec![]);
}

impl<S, E> FmtSubscriber<S, E> {
    fn span_name(&self, id: &Id) -> Option<&'static str> {
        self.spans.read().ok()?
            .get(id).map(|span| span.name)
    }

    fn enter(&self, id: &Id) {
        if let Some(span_name) = self.span_name(id) {
            let _ = CONTEXT.try_with(|current| {
                current.borrow_mut().push(span_name);
            });
        }
    }

    fn exit(&self) {
        // TODO: do we need to actually make sure we popped the right thing
        // here? or can we rely on `tokio-trace` to be sane?
        let _ = CONTEXT.try_with(|current| {
            current.borrow_mut().pop()
        });
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
        let data = SpanData {
            name: metadata.name(),
        };
        self.spans.write().expect("rwlock poisoned!")
            .insert(id.clone(), data);
        id
    }

    fn record(&self, span: &Id, values: &field::ValueSet) {}

    fn record_follows_from(&self, span: &Id, follows: &Id) {}

    fn event(&self, event: &Event) {
        // TODO: better io generation!
        let stdout = io::stdout();
        let mut io = stdout.lock();
        if let Err(e) = (self.fmt_event)(Context::new(), &mut io, event) {
            eprintln!("error formatting event: {}", e);
        }
    }

    fn enter(&self, span: &Id) {
        // TODO: add on_enter hook
        self.enter(span);
    }

    fn exit(&self, _span: &Id)  {
        // TODO: add on_exit hook
        self.exit();
    }
}


impl Context {
    pub fn fmt<F>(&self, f: F) -> fmt::Result
    where
        F: FnOnce(&[&str]) -> fmt::Result
    {
        CONTEXT.try_with(|current| {
            (f)(&current.borrow()[..])
        }).map_err(|_| fmt::Error)?
    }

    fn new() -> Self {
        Self {
            _p: ()
        }
    }
}
