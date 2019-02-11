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
    marker::PhantomData,
};

pub mod default;
mod span;

#[derive(Debug)]
pub struct FmtSubscriber<
    N = default::NewRecorder,
    E = fn(&Context, &mut io::Write, &Event) -> io::Result<()>,
> {
    new_recorder: N,
    fmt_event: E,
    spans: RwLock<HashMap<Id, span::Data>>,
    next_id: AtomicUsize,
}

pub struct Context<'a> {
    lock: &'a RwLock<HashMap<Id, span::Data>>,
}

#[derive(Debug, Default)]
pub struct Builder<
    N = default::NewRecorder,
    E = fn(&Context, &mut io::Write, &Event) -> io::Result<()>,
> {
    new_recorder: N
    fmt_event: E
}

thread_local! {
    static CONTEXT: RefCell<Vec<Id>> = RefCell::new(vec![]);
}

fn curr_id() -> Option<Id> {
    CONTEXT.try_with(|current| current.borrow().last().cloned()).ok()?
}

impl Default for Builder {
    fn default() -> Self {
        Builder {
            new_recorder: default::NewRecorder,
            fmt_event: default::fmt_event,
        }
    }
}

impl<N, E> Builder<N, E> {
    pub fn with_recorder<N2>(self, new_recorder: N2) -> Builder<N2, E>
    where
        N2: for<'a> NewRecorder<'a>,
    {
        Builder {
            new_recorder,
            fmt_event: self.fmt_event,
        }
    }

    pub fn on_event<E2>(self, fmt_event: E2) -> Builder<N, E2>
    where
        E2: Fn(&Context, &mut io::Write, &Event) -> io::Result<()>,
    {
        Builder {
            new_recorder: self.new_recorder,
            fmt_event,
        }
    }

}

impl<N, E> Builder<N, E>
where
    N: for<'a> NewRecorder<'a>,
    E: Fn(&Context, &mut io::Write, &Event) -> io::Result<()>,
{
    pub fn finish(self) -> FmtSubscriber<N, E> {
        FmtSubscriber {
            new_recorder: self.new_recorder,
            fmt_event: self.fmt_event,
            spans: RwLock::new(HashMap::default()),
            next_id: AtomicUsize::new(0),
        }
    }
}

impl<N, E> FmtSubscriber<N, E> {
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
        Builder::default().finish()
    }
}

impl<N, E> tokio_trace_core::Subscriber for FmtSubscriber<N, E>
where
    N: for<'a> NewRecorder<'a>,
    E: Fn(&Context, &mut io::Write, &Event) -> io::Result<()>,
{
   fn enabled(&self, metadata: &Metadata) -> bool {
       // FIXME: filtering
       true
   }

    fn new_span(&self, metadata: &Metadata, values: &field::ValueSet) -> Id {
        let id = Id::from_u64(self.next_id.fetch_add(1, Ordering::Relaxed) as u64);
        let mut data = span::Data::new(metadata.name());
        {
            let mut recorder = self.new_recorder.make(&mut data);
            values.record(&mut recorder);
        }
        self.spans.write().expect("rwlock poisoned!")
            .insert(id.clone(), data);
        id
    }

    fn record(&self, span: &Id, values: &field::ValueSet) {
        let mut spans = self.spans.write().expect("rwlock poisoned!");
        if let Some(mut span) = spans.get_mut(span) {

            let mut recorder = self.new_recorder.make(&mut span);
            values.record(&mut recorder);
        }
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {}

    fn event(&self, event: &Event) {
        use io::Write;
        if let Err(e) = (|| -> io::Result<()> {
            let stdout = io::stdout();
            let mut io = stdout.lock();
            let ctx = Context::new(&self.spans);
            let _ = {
                (self.fmt_event)(&ctx, &mut io, event)
            }?;
            {
                let mut recorder = self.new_recorder.make(&mut io);
                event.record(&mut recorder);
            }
            let _ = ctx.fmt_fields(&mut io)?;
            let _ = io.write(b"\n")?;
            Ok(())
        })() {
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

    fn fmt_fields<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        curr_id().map(|id| {
            if let Some(span) = self.lock.read()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, "rwlock poisoned"))?
                .get(&id)
            {
                write!(writer, "{}", span.fields)
            } else {
                Ok(())
            }
        }).unwrap_or(Ok(()))
    }

    fn new(lock: &'a RwLock<HashMap<Id, span::Data>>) -> Self {
        Self {
            lock,
        }
    }
}


pub trait NewRecorder<'a> {
    type Recorder: field::Record + 'a;

    fn make(&self, writer: &'a mut io::Write) -> Self::Recorder;
}

impl<'a, F, R> NewRecorder<'a> for F
where
    F: Fn(&'a mut io::Write) -> R,
    R: field::Record + 'a,
{
    type Recorder = R;

    #[inline]
    fn make(&self, writer: &'a mut io::Write) -> Self::Recorder {
        (self)(writer)
    }
}
