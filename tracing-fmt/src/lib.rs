extern crate tracing_core;
#[cfg(test)]
#[macro_use]
extern crate tracing;

#[cfg(feature = "ansi")]
extern crate ansi_term;
extern crate lock_api;
extern crate owning_ref;
extern crate parking_lot;

#[macro_use]
extern crate lazy_static;
extern crate regex;

use tracing_core::{field, subscriber::Interest, Event, Metadata};

use std::{any::TypeId, cell::RefCell, fmt, io};

pub mod default;
pub mod filter;
mod span;

pub use filter::Filter;
pub use span::Context;

#[derive(Debug)]
pub struct FmtSubscriber<
    N = default::NewRecorder,
    E = fn(&span::Context<N>, &mut dyn fmt::Write, &Event) -> fmt::Result,
    F = filter::EnvFilter,
> {
    new_visitor: N,
    fmt_event: E,
    filter: F,
    spans: span::Store,
    settings: Settings,
}

#[derive(Debug, Default)]
pub struct Builder<
    N = default::NewRecorder,
    E = fn(&span::Context<N>, &mut dyn fmt::Write, &Event) -> fmt::Result,
    F = filter::EnvFilter,
> {
    new_visitor: N,
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

impl<N, E, F> FmtSubscriber<N, E, F>
where
    N: for<'a> NewVisitor<'a>,
{
    #[inline]
    fn ctx(&self) -> span::Context<N> {
        span::Context::new(&self.spans, &self.new_visitor)
    }
}

impl<N, E, F> FmtSubscriber<N, E, filter::ReloadFilter<F, N>>
where
    F: Filter<N> + 'static,
{
    /// Returns a `Handle` that may be used to reload this subscriber's
    /// filter.
    pub fn reload_handle(&self) -> filter::reload::Handle<F, N> {
        self.filter.handle()
    }
}

impl<N, E, F> tracing_core::Subscriber for FmtSubscriber<N, E, F>
where
    N: for<'a> NewVisitor<'a> + 'static,
    E: Fn(&span::Context<N>, &mut dyn fmt::Write, &Event) -> fmt::Result + 'static,
    F: Filter<N> + 'static,
{
    fn register_callsite(&self, metadata: &Metadata) -> Interest {
        self.filter.callsite_enabled(metadata, &self.ctx())
    }

    fn enabled(&self, metadata: &Metadata) -> bool {
        self.filter.enabled(metadata, &self.ctx())
    }

    #[inline]
    fn new_span(&self, attrs: &span::Attributes) -> span::Id {
        let span = span::Data::new(attrs);
        self.spans.new_span(span, attrs, &self.new_visitor)
    }

    #[inline]
    fn record(&self, span: &span::Id, values: &span::Record) {
        self.spans.record(span, values, &self.new_visitor)
    }

    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {
        // TODO: implement this please
    }

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
            let ctx = span::Context::new(&self.spans, &self.new_visitor);

            if (self.fmt_event)(&ctx, buf, event).is_ok() {
                // TODO: make the io object configurable
                let _ = io::Write::write_all(&mut io::stdout(), buf.as_bytes());
            }

            buf.clear();
        });
    }

    fn enter(&self, id: &span::Id) {
        // TODO: add on_enter hook
        span::push(id);
    }

    fn exit(&self, id: &span::Id) {
        span::pop(id);
    }

    fn clone_span(&self, id: &span::Id) -> span::Id {
        if let Some(span) = self.spans.get(id) {
            span.clone_ref()
        }
        id.clone()
    }

    fn drop_span(&self, id: span::Id) {
        self.spans.drop_span(id);
    }

    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        match () {
            _ if id == TypeId::of::<Self>() => Some(self as *const Self as *const ()),
            _ if id == TypeId::of::<F>() => Some(&self.filter as *const F as *const ()),
            _ if id == TypeId::of::<E>() => Some(&self.fmt_event as *const E as *const ()),
            _ if id == TypeId::of::<N>() => Some(&self.new_visitor as *const N as *const ()),
            _ => None,
        }
    }
}

pub trait NewVisitor<'a> {
    type Visitor: field::Visit + 'a;

    fn make(&self, writer: &'a mut dyn fmt::Write, is_empty: bool) -> Self::Visitor;
}

impl<'a, F, R> NewVisitor<'a> for F
where
    F: Fn(&'a mut dyn fmt::Write, bool) -> R,
    R: field::Visit + 'a,
{
    type Visitor = R;

    #[inline]
    fn make(&self, writer: &'a mut dyn fmt::Write, is_empty: bool) -> Self::Visitor {
        (self)(writer, is_empty)
    }
}

// ===== impl Builder =====

impl Default for Builder {
    fn default() -> Self {
        Builder {
            filter: filter::EnvFilter::from_default_env(),
            new_visitor: default::NewRecorder,
            fmt_event: default::fmt_event,
            settings: Settings::default(),
        }
    }
}

impl<N, E, F> Builder<N, E, F>
where
    N: for<'a> NewVisitor<'a> + 'static,
    E: Fn(&span::Context<N>, &mut dyn fmt::Write, &Event) -> fmt::Result + 'static,
    F: Filter<N> + 'static,
{
    pub fn finish(self) -> FmtSubscriber<N, E, F> {
        FmtSubscriber {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event,
            filter: self.filter,
            spans: span::Store::with_capacity(32),
            settings: self.settings,
        }
    }
}

impl<N, E, F> Builder<N, E, F>
where
    F: Filter<N> + 'static,
{
    /// Configures the subscriber being built to allow filter reloading at
    /// runtime.
    pub fn with_filter_reloading(self) -> Builder<N, E, filter::ReloadFilter<F, N>> {
        Builder {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event,
            filter: filter::ReloadFilter::new(self.filter),
            settings: self.settings,
        }
    }
}

impl<N, E, F> Builder<N, E, filter::ReloadFilter<F, N>>
where
    F: Filter<N> + 'static,
{
    /// Returns a `Handle` that may be used to reload the constructed subscriber's
    /// filter.
    pub fn reload_handle(&self) -> filter::reload::Handle<F, N> {
        self.filter.handle()
    }
}

impl<N, E, F> Builder<N, E, F> {
    /// Sets the Visitor that the subscriber being built will use to record
    /// fields.
    pub fn with_visitor<N2>(self, new_visitor: N2) -> Builder<N2, E, F>
    where
        N2: for<'a> NewVisitor<'a> + 'static,
    {
        Builder {
            new_visitor,
            fmt_event: self.fmt_event,
            filter: self.filter,
            settings: self.settings,
        }
    }

    /// Sets the filter that the subscriber being built will use to determine if
    /// a span or event is enabled.
    pub fn with_filter<F2>(self, filter: F2) -> Builder<N, E, F2>
    where
        F2: Filter<N> + 'static,
    {
        Builder {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event,
            filter,
            settings: self.settings,
        }
    }

    /// Sets the subscriber being built to use the default full span formatter.
    // TODO: this should probably just become the default.
    pub fn full(
        self,
    ) -> Builder<N, fn(&span::Context<N>, &mut dyn fmt::Write, &Event) -> fmt::Result, F>
    where
        N: for<'a> NewVisitor<'a> + 'static,
    {
        Builder {
            fmt_event: default::fmt_verbose,
            filter: self.filter,
            new_visitor: self.new_visitor,
            settings: self.settings,
        }
    }

    /// Sets the function that the subscriber being built should use to format
    /// events that occur.
    pub fn on_event<E2>(self, fmt_event: E2) -> Builder<N, E2, F>
    where
        E2: Fn(&span::Context<N>, &mut dyn fmt::Write, &Event) -> fmt::Result + 'static,
    {
        Builder {
            new_visitor: self.new_visitor,
            fmt_event,
            filter: self.filter,
            settings: self.settings,
        }
    }

    /// Sets whether or not spans inherit their parents' field values (disabled
    /// by default).
    pub fn inherit_fields(self, inherit_fields: bool) -> Self {
        Builder {
            settings: Settings { inherit_fields },
            ..self
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fmt;
    use tracing_core::Dispatch;

    #[test]
    fn subscriber_downcasts() {
        let subscriber = FmtSubscriber::new();
        let dispatch = Dispatch::new(subscriber);
        assert!(dispatch.downcast_ref::<FmtSubscriber>().is_some());
    }

    #[test]
    fn subscriber_downcasts_to_parts() {
        type FmtEvent =
            fn(&span::Context<default::NewRecorder>, &mut dyn fmt::Write, &Event) -> fmt::Result;
        let subscriber = FmtSubscriber::new();
        let dispatch = Dispatch::new(subscriber);
        assert!(dispatch.downcast_ref::<default::NewRecorder>().is_some());
        assert!(dispatch.downcast_ref::<filter::EnvFilter>().is_some());
        assert!(dispatch.downcast_ref::<FmtEvent>().is_some())
    }
}
