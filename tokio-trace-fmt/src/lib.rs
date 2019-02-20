extern crate tokio_trace_core;

#[cfg(feature = "ansi")]
extern crate ansi_term;

use tokio_trace_core::{field, subscriber::Interest, Event, Metadata};

use std::{cell::RefCell, fmt, io, sync::RwLock};

pub mod default;
pub mod filter;
mod span;

pub use filter::Filter;

#[derive(Debug)]
pub struct FmtSubscriber<
    N = default::NewRecorder,
    E = fn(&span::Context, &mut fmt::Write, &Event) -> fmt::Result,
    F = filter::EnvFilter,
> {
    new_recorder: N,
    fmt_event: E,
    filter: F,
    spans: RwLock<span::Slab>,
    settings: Settings,
}

#[derive(Debug, Default)]
pub struct Builder<
    N = default::NewRecorder,
    E = fn(&span::Context, &mut fmt::Write, &Event) -> fmt::Result,
    F = filter::EnvFilter,
> {
    new_recorder: N,
    fmt_event: E,
    filter: F,
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

impl<N, E, F> FmtSubscriber<N, E, F> {
    #[inline]
    fn ctx(&self) -> span::Context {
        span::Context::new(&self.spans)
    }
}

impl<N, E, F> tokio_trace_core::Subscriber for FmtSubscriber<N, E, F>
where
    N: for<'a> NewRecorder<'a>,
    E: Fn(&span::Context, &mut fmt::Write, &Event) -> fmt::Result,
    F: Filter,
{
    fn register_callsite(&self, metadata: &Metadata) -> Interest {
        self.filter.callsite_enabled(metadata, &self.ctx())
    }

    fn enabled(&self, metadata: &Metadata) -> bool {
        self.filter.enabled(metadata, &self.ctx())
    }

    fn new_span(&self, metadata: &Metadata, values: &field::ValueSet) -> span::Id {
        let span = span::Data::new(metadata);
        self.spans
            .write()
            .expect("rwlock poisoned!")
            .new_span(span, values, &self.new_recorder)
    }

    fn record(&self, span: &span::Id, values: &field::ValueSet) {
        self.spans
            .write()
            .expect("rwlock poisoned!")
            .record(span, values, &self.new_recorder)
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {}

    fn event(&self, event: &Event) {
        thread_local! {
            static BUF: RefCell<String> = RefCell::new(String::new());
        }

        BUF.with(|buf| {
            let borrow = buf.try_borrow_mut();
            let mut a;
            let mut b;
            let buf = match borrow {
                Ok(buf) => {
                    a = buf;
                    &mut *a
                }
                _ => {
                    b = String::new();
                    &mut b
                }
            };
            let ctx = span::Context::new(&self.spans);

            if (self.fmt_event)(&ctx, buf, event).is_ok() {
                // TODO: make the io object configurable
                let _ = io::Write::write_all(&mut io::stdout(), buf.as_bytes());
            }

            buf.clear();
        });
    }

    fn enter(&self, span: &span::Id) {
        // TODO: add on_enter hook
        span::Context::push(self.clone_span(span));
    }

    fn exit(&self, span: &span::Id) {
        // TODO: add on_exit hook
        if let Some(popped) = span::Context::pop() {
            debug_assert!(&popped == span);
            self.drop_span(popped);
        }
    }

    fn clone_span(&self, id: &span::Id) -> span::Id {
        if let Ok(spans) = self.spans.read() {
            if let Some(span) = spans.get(id) {
                span.clone_ref()
            }
        }
        id.clone()
    }

    fn drop_span(&self, id: span::Id) {
        if self
            .spans
            .read()
            .ok()
            .and_then(|spans| spans.get(&id).map(|span| span.drop_ref()))
            .unwrap_or(false)
        {
            if let Ok(mut spans) = self.spans.write() {
                spans.remove(&id);
            }
        }
    }
}

pub trait NewRecorder<'a> {
    type Recorder: field::Record + 'a;

    fn make(&self, writer: &'a mut fmt::Write, is_empty: bool) -> Self::Recorder;
}

impl<'a, F, R> NewRecorder<'a> for F
where
    F: Fn(&'a mut fmt::Write, bool) -> R,
    R: field::Record + 'a,
{
    type Recorder = R;

    #[inline]
    fn make(&self, writer: &'a mut fmt::Write, is_empty: bool) -> Self::Recorder {
        (self)(writer, is_empty)
    }
}

// ===== impl Builder =====

impl Default for Builder {
    fn default() -> Self {
        Builder {
            filter: filter::EnvFilter::from_default_env(),
            new_recorder: default::NewRecorder,
            fmt_event: default::fmt_event,
            settings: Settings::default(),
        }
    }
}

impl<N, E, F> Builder<N, E, F>
where
    N: for<'a> NewRecorder<'a>,
    E: Fn(&span::Context, &mut fmt::Write, &Event) -> fmt::Result,
    F: Filter,
{
    pub fn finish(self) -> FmtSubscriber<N, E, F> {
        FmtSubscriber {
            new_recorder: self.new_recorder,
            fmt_event: self.fmt_event,
            filter: self.filter,
            spans: RwLock::new(span::Slab::with_capacity(32)),
            settings: self.settings,
        }
    }
}

impl<N, E, F> Builder<N, E, F> {
    pub fn with_recorder<N2>(self, new_recorder: N2) -> Builder<N2, E, F>
    where
        N2: for<'a> NewRecorder<'a>,
    {
        Builder {
            new_recorder,
            fmt_event: self.fmt_event,
            filter: self.filter,
            settings: self.settings,
        }
    }

    pub fn with_filter<F2>(self, filter: F2) -> Builder<N, E, F2>
    where
        F2: Filter,
    {
        Builder {
            new_recorder: self.new_recorder,
            fmt_event: self.fmt_event,
            filter,
            settings: self.settings,
        }
    }

    pub fn full(self) -> Builder<N, fn(&span::Context, &mut fmt::Write, &Event) -> fmt::Result, F> {
        Builder {
            fmt_event: default::fmt_verbose,
            filter: self.filter,
            new_recorder: self.new_recorder,
            settings: self.settings,
        }
    }

    pub fn on_event<E2>(self, fmt_event: E2) -> Builder<N, E2, F>
    where
        E2: Fn(&span::Context, &mut fmt::Write, &Event) -> fmt::Result,
    {
        Builder {
            new_recorder: self.new_recorder,
            fmt_event,
            filter: self.filter,
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
