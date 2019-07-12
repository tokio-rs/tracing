extern crate tracing_core;
#[cfg(test)]
#[macro_use]
extern crate tracing;

#[cfg(feature = "ansi")]
extern crate ansi_term;
#[cfg(feature = "chrono")]
extern crate chrono;
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
pub mod time;

pub use crate::filter::Filter;
pub use crate::span::Context;

/// A type that can format a tracing `Event` for a `fmt::Write`.
///
/// `FormatEvent` is primarily used in the context of [`FmtSubscriber`]. Each time an event is
/// dispatched to [`FmtSubscriber`], the subscriber forwards it to its associated `FormatEvent` to
/// emit a log message.
///
/// This trait is already implemented for function pointers with the same signature as `format`.
pub trait FormatEvent<N> {
    /// Write a log message for `Event` in `Context` to the given `Write`.
    fn format_event(
        &self,
        ctx: &span::Context<N>,
        writer: &mut dyn fmt::Write,
        event: &Event,
    ) -> fmt::Result;
}

impl<N> FormatEvent<N> for fn(&span::Context<N>, &mut dyn fmt::Write, &Event) -> fmt::Result {
    fn format_event(
        &self,
        ctx: &span::Context<N>,
        writer: &mut dyn fmt::Write,
        event: &Event,
    ) -> fmt::Result {
        (*self)(ctx, writer, event)
    }
}

#[derive(Debug)]
pub struct FmtSubscriber<
    N = default::NewRecorder,
    E = default::Format<default::Full>,
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
    E = default::Format<default::Full>,
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
    E: FormatEvent<N> + 'static,
    F: Filter<N> + 'static,
{
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        self.filter.callsite_enabled(metadata, &self.ctx())
    }

    fn enabled(&self, metadata: &Metadata) -> bool {
        self.filter.enabled(metadata, &self.ctx())
    }

    #[inline]
    fn new_span(&self, attrs: &span::Attributes) -> span::Id {
        self.spans.new_span(attrs, &self.new_visitor)
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

            if self.fmt_event.format_event(&ctx, buf, event).is_ok() {
                // TODO: make the io object configurable
                let _ = io::Write::write_all(&mut io::stdout(), buf.as_bytes());
            }

            buf.clear();
        });
    }

    fn enter(&self, id: &span::Id) {
        // TODO: add on_enter hook
        self.spans.push(id);
    }

    fn exit(&self, id: &span::Id) {
        self.spans.pop(id);
    }

    fn current_span(&self) -> span::Current {
        if let Some(id) = self.spans.current() {
            if let Some(meta) = self.spans.get(&id).map(|span| span.metadata()) {
                return span::Current::new(id, meta);
            }
        }
        span::Current::none()
    }

    #[inline]
    fn clone_span(&self, id: &span::Id) -> span::Id {
        self.spans.clone_span(id)
    }

    #[inline]
    fn try_close(&self, id: span::Id) -> bool {
        self.spans.drop_span(id)
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
            fmt_event: default::Format::default(),
            settings: Settings::default(),
        }
    }
}

impl<N, E, F> Builder<N, E, F>
where
    N: for<'a> NewVisitor<'a> + 'static,
    E: FormatEvent<N> + 'static,
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

impl<N, L, T, F> Builder<N, default::Format<L, T>, F>
where
    N: for<'a> NewVisitor<'a> + 'static,
    F: Filter<N> + 'static,
{
    /// Use the given `timer` for log message timestamps.
    pub fn with_timer<T2>(self, timer: T2) -> Builder<N, default::Format<L, T2>, F> {
        Builder {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event.with_timer(timer),
            filter: self.filter,
            settings: self.settings,
        }
    }

    /// Use the given `timer` for log message timestamps.
    pub fn without_time(self) -> Builder<N, default::Format<L, ()>, F> {
        Builder {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event.without_time(),
            filter: self.filter,
            settings: self.settings,
        }
    }

    /// Enable ANSI encoding for formatted events.
    #[cfg(feature = "ansi")]
    pub fn with_ansi(self, ansi: bool) -> Builder<N, default::Format<L, T>, F> {
        Builder {
            fmt_event: self.fmt_event.with_ansi(ansi),
            ..self
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

    /// Sets the subscriber being built to use a less verbose formatter.
    ///
    /// See [`default::Compact`].
    pub fn compact(self) -> Builder<N, default::Format<default::Compact>, F>
    where
        N: for<'a> NewVisitor<'a> + 'static,
    {
        Builder {
            fmt_event: default::Format::default().compact(),
            filter: self.filter,
            new_visitor: self.new_visitor,
            settings: self.settings,
        }
    }

    /// Sets the function that the subscriber being built should use to format
    /// events that occur.
    pub fn on_event<E2>(self, fmt_event: E2) -> Builder<N, E2, F>
    where
        E2: FormatEvent<N> + 'static,
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
    use tracing_core::dispatcher::Dispatch;

    #[test]
    fn impls() {
        let f = default::Format::default().with_timer(time::Uptime::default());
        let subscriber = FmtSubscriber::builder().on_event(f).finish();
        let _dispatch = Dispatch::new(subscriber);

        let f = default::Format::default();
        let subscriber = FmtSubscriber::builder().on_event(f).finish();
        let _dispatch = Dispatch::new(subscriber);

        let f = default::Format::default().compact();
        let subscriber = FmtSubscriber::builder().on_event(f).finish();
        let _dispatch = Dispatch::new(subscriber);
    }

    #[test]
    fn subscriber_downcasts() {
        let subscriber = FmtSubscriber::new();
        let dispatch = Dispatch::new(subscriber);
        assert!(dispatch.downcast_ref::<FmtSubscriber>().is_some());
    }

    #[test]
    fn subscriber_downcasts_to_parts() {
        let subscriber = FmtSubscriber::new();
        let dispatch = Dispatch::new(subscriber);
        assert!(dispatch.downcast_ref::<default::NewRecorder>().is_some());
        assert!(dispatch.downcast_ref::<filter::EnvFilter>().is_some());
        assert!(dispatch.downcast_ref::<default::Format>().is_some())
    }
}
