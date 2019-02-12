extern crate tokio_trace_core;

#[cfg(feature = "ansi")]
extern crate ansi_term;

use tokio_trace_core::{
    field,
    Event,
    Metadata,
};

use std::{
    fmt,
    io,
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        RwLock
    },
};

pub mod default;
mod span;

#[derive(Debug)]
pub struct FmtSubscriber<
    N = default::NewRecorder,
    E = fn(&span::Context, &mut io::Write, &Event) -> io::Result<()>,
> {
    new_recorder: N,
    fmt_event: E,
    spans: RwLock<HashMap<span::Id, span::Data>>,
    next_id: AtomicUsize,
    settings: Settings,
}

#[derive(Debug, Default)]
pub struct Builder<
    N = default::NewRecorder,
    E = fn(&span::Context, &mut io::Write, &Event) -> io::Result<()>,
> {
    new_recorder: N,
    fmt_event: E,
    settings: Settings,
}

#[derive(Debug, Default)]
struct Settings {
    inherit_fields: bool,
}

impl FmtSubscriber {
    pub fn builder() -> Builder {
        Builder::default()
    }

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
    E: Fn(&span::Context, &mut io::Write, &Event) -> io::Result<()>,
{
   fn enabled(&self, metadata: &Metadata) -> bool {
       // FIXME: filtering
       true
   }

    fn new_span(&self, metadata: &Metadata, values: &field::ValueSet) -> span::Id {
        let id = span::Id::from_u64(self.next_id.fetch_add(1, Ordering::Relaxed) as u64);
        let fields =
            if self.settings.inherit_fields {
                span::Context::new(&self.spans)
                    .with_current(|(_, span)| span.fields.to_owned())
                    .unwrap_or_default()
            } else {
                String::new()
            };
        let mut data = span::Data::new(metadata.name(), fields);
        {
            let mut recorder = self.new_recorder.make(&mut data);
            values.record(&mut recorder);
        }
        self.spans.write().expect("rwlock poisoned!")
            .insert(id.clone(), data);
        id
    }

    fn record(&self, span: &span::Id, values: &field::ValueSet) {
        let mut spans = self.spans.write().expect("rwlock poisoned!");
        if let Some(mut span) = spans.get_mut(span) {

            let mut recorder = self.new_recorder.make(&mut span);
            values.record(&mut recorder);
        }
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {}

    fn event(&self, event: &Event) {
        // TODO: we should probably pass in a buffered writer type, and
        // allow alternate IOs...
        let stdout = io::stdout();
        let mut io = stdout.lock();
        let ctx = span::Context::new(&self.spans);
        if let Err(e) = (self.fmt_event)(&ctx, &mut io, event) {
            eprintln!("error formatting event: {}", e);
        }
    }

    fn enter(&self, span: &span::Id) {
        // TODO: add on_enter hook
        span::Context::push(span.clone());
    }

    fn exit(&self, span: &span::Id)  {
        // TODO: add on_exit hook
        if let Some(ref popped) = span::Context::pop() {
            debug_assert!(popped == span);
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

// ===== impl Builder =====

impl Default for Builder {
    fn default() -> Self {
        Builder {
            new_recorder: default::NewRecorder,
            fmt_event: default::fmt_event,
            settings: Settings::default(),
        }
    }
}

impl<N, E> Builder<N, E>
where
    N: for<'a> NewRecorder<'a>,
    E: Fn(&span::Context, &mut io::Write, &Event) -> io::Result<()>,
{
    pub fn finish(self) -> FmtSubscriber<N, E> {
        FmtSubscriber {
            new_recorder: self.new_recorder,
            fmt_event: self.fmt_event,
            spans: RwLock::new(HashMap::default()),
            next_id: AtomicUsize::new(0),
            settings: self.settings,
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
            settings: self.settings,
        }
    }

    pub fn on_event<E2>(self, fmt_event: E2) -> Builder<N, E2>
    where
        E2: Fn(&span::Context, &mut io::Write, &Event) -> io::Result<()>,
    {
        Builder {
            new_recorder: self.new_recorder,
            fmt_event,
            settings: self.settings,
        }
    }

    /// Sets whether or not spans inherit their parents' field values (disabled
    /// by default).
    pub fn inherit_fields(self, inherit_fields: bool) -> Self {
        Builder {
            settings: Settings {
                inherit_fields,
                ..self.settings
            },
            ..self
        }
    }

}
