//! A `Subscriber` for formatting and logging `tracing` data.
//!
//! ## Overview
//!
//! [`tracing`] is a framework for instrumenting Rust programs with context-aware,
//! structured, event-based diagnostic information. This crate provides an
//! implementation of the [`Subscriber`] trait that records `tracing`'s `Event`s
//! and `Span`s by formatting them as text and logging them to stdout.
//!
//!
//! [`tracing`]: https://crates.io/crates/tracing
//! [`Subscriber`]: https://docs.rs/tracing/latest/tracing/trait.Subscriber.html
#![doc(html_root_url = "https://docs.rs/tracing-f,t/0.0.1-alpha.3")]
#![cfg_attr(test, deny(warnings))]
use tracing_core::{field, subscriber::Interest, Event, Metadata};

use std::{any::TypeId, cell::RefCell, fmt, io};

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

pub mod filter;
pub mod format;
mod span;
pub mod time;
pub mod writer;

#[doc(inline)]
pub use crate::{filter::Filter, format::FormatEvent, span::Context, writer::MakeWriter};

/// A `Subscriber` that logs formatted representations of `tracing` events.
#[derive(Debug)]
pub struct FmtSubscriber<
    N = format::NewRecorder,
    E = format::Format<format::Full>,
    F = filter::EnvFilter,
    W = fn() -> io::Stdout,
> {
    new_visitor: N,
    fmt_event: E,
    filter: F,
    spans: span::Store,
    settings: Settings,
    make_writer: W,
}

/// Configures and constructs `FmtSubscriber`s.
#[derive(Debug, Default)]
pub struct Builder<
    N = format::NewRecorder,
    E = format::Format<format::Full>,
    F = filter::EnvFilter,
    W = fn() -> io::Stdout,
> {
    new_visitor: N,
    fmt_event: E,
    filter: F,
    settings: Settings,
    make_writer: W,
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

impl<N, E, F, W> FmtSubscriber<N, E, F, W>
where
    N: for<'a> NewVisitor<'a>,
{
    #[inline]
    fn ctx(&self) -> span::Context<'_, N> {
        span::Context::new(&self.spans, &self.new_visitor)
    }
}

impl<N, E, F, W> FmtSubscriber<N, E, filter::ReloadFilter<F, N>, W>
where
    F: Filter<N> + 'static,
{
    /// Returns a `Handle` that may be used to reload this subscriber's
    /// filter.
    pub fn reload_handle(&self) -> filter::reload::Handle<F, N> {
        self.filter.handle()
    }
}

impl<N, E, F, W> tracing_core::Subscriber for FmtSubscriber<N, E, F, W>
where
    N: for<'a> NewVisitor<'a> + 'static,
    E: FormatEvent<N> + 'static,
    F: Filter<N> + 'static,
    W: MakeWriter + 'static,
{
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        self.filter.callsite_enabled(metadata, &self.ctx())
    }

    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.filter.enabled(metadata, &self.ctx())
    }

    #[inline]
    fn new_span(&self, attrs: &span::Attributes<'_>) -> span::Id {
        self.spans.new_span(attrs, &self.new_visitor)
    }

    #[inline]
    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        self.spans.record(span, values, &self.new_visitor)
    }

    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {
        // TODO: implement this please
    }

    fn event(&self, event: &Event<'_>) {
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
                let mut writer = self.make_writer.make_writer();
                let _ = io::Write::write_all(&mut writer, buf.as_bytes());
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
            new_visitor: format::NewRecorder,
            fmt_event: format::Format::default(),
            settings: Settings::default(),
            make_writer: io::stdout,
        }
    }
}

impl<N, E, F, W> Builder<N, E, F, W>
where
    N: for<'a> NewVisitor<'a> + 'static,
    E: FormatEvent<N> + 'static,
    F: Filter<N> + 'static,
    W: MakeWriter + 'static,
{
    pub fn finish(self) -> FmtSubscriber<N, E, F, W> {
        FmtSubscriber {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event,
            filter: self.filter,
            spans: span::Store::with_capacity(32),
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }
}

impl<N, L, T, F, W> Builder<N, format::Format<L, T>, F, W>
where
    N: for<'a> NewVisitor<'a> + 'static,
    F: Filter<N> + 'static,
{
    /// Use the given `timer` for log message timestamps.
    pub fn with_timer<T2>(self, timer: T2) -> Builder<N, format::Format<L, T2>, F, W> {
        Builder {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event.with_timer(timer),
            filter: self.filter,
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }

    /// Do not emit timestamps with log messages.
    pub fn without_time(self) -> Builder<N, format::Format<L, ()>, F, W> {
        Builder {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event.without_time(),
            filter: self.filter,
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }

    /// Enable ANSI encoding for formatted events.
    #[cfg(feature = "ansi")]
    pub fn with_ansi(self, ansi: bool) -> Builder<N, format::Format<L, T>, F, W> {
        Builder {
            fmt_event: self.fmt_event.with_ansi(ansi),
            ..self
        }
    }

    /// Sets whether or not an event's target is displayed.
    pub fn with_target(self, display_target: bool) -> Builder<N, format::Format<L, T>, F, W> {
        Builder {
            fmt_event: self.fmt_event.with_target(display_target),
            ..self
        }
    }
}

impl<N, E, F, W> Builder<N, E, F, W>
where
    F: Filter<N> + 'static,
{
    /// Configures the subscriber being built to allow filter reloading at
    /// runtime.
    pub fn with_filter_reloading(self) -> Builder<N, E, filter::ReloadFilter<F, N>, W> {
        Builder {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event,
            filter: filter::ReloadFilter::new(self.filter),
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }
}

impl<N, E, F, W> Builder<N, E, filter::ReloadFilter<F, N>, W>
where
    F: Filter<N> + 'static,
{
    /// Returns a `Handle` that may be used to reload the constructed subscriber's
    /// filter.
    pub fn reload_handle(&self) -> filter::reload::Handle<F, N> {
        self.filter.handle()
    }
}

impl<N, E, F, W> Builder<N, E, F, W> {
    /// Sets the Visitor that the subscriber being built will use to record
    /// fields.
    pub fn with_visitor<N2>(self, new_visitor: N2) -> Builder<N2, E, F, W>
    where
        N2: for<'a> NewVisitor<'a> + 'static,
    {
        Builder {
            new_visitor,
            fmt_event: self.fmt_event,
            filter: self.filter,
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }

    /// Sets the filter that the subscriber being built will use to determine if
    /// a span or event is enabled.
    pub fn with_filter<F2>(self, filter: F2) -> Builder<N, E, F2, W>
    where
        F2: Filter<N> + 'static,
    {
        Builder {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event,
            filter,
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }

    /// Sets the subscriber being built to use a less verbose formatter.
    ///
    /// See [`format::Compact`].
    pub fn compact(self) -> Builder<N, format::Format<format::Compact>, F, W>
    where
        N: for<'a> NewVisitor<'a> + 'static,
    {
        Builder {
            fmt_event: format::Format::default().compact(),
            filter: self.filter,
            new_visitor: self.new_visitor,
            settings: self.settings,
            make_writer: self.make_writer,
        }
    }

    /// Sets the function that the subscriber being built should use to format
    /// events that occur.
    pub fn on_event<E2>(self, fmt_event: E2) -> Builder<N, E2, F, W>
    where
        E2: FormatEvent<N> + 'static,
    {
        Builder {
            new_visitor: self.new_visitor,
            fmt_event,
            filter: self.filter,
            settings: self.settings,
            make_writer: self.make_writer,
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

impl<N, E, F, W> Builder<N, E, F, W> {
    /// Sets the [`MakeWriter`] that the subscriber being built will use to write events.
    ///
    /// # Examples
    ///
    /// Using `stderr` rather than `stdout`:
    ///
    /// ```rust
    /// use std::io;
    ///
    /// let subscriber = tracing_fmt::FmtSubscriber::builder()
    ///     .with_writer(io::stderr)
    ///     .finish();
    /// ```
    ///
    /// [`MakeWriter`]: trait.MakeWriter.html
    pub fn with_writer<W2>(self, make_writer: W2) -> Builder<N, E, F, W2>
    where
        W2: MakeWriter + 'static,
    {
        Builder {
            new_visitor: self.new_visitor,
            fmt_event: self.fmt_event,
            filter: self.filter,
            settings: self.settings,
            make_writer,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tracing_core::dispatcher::Dispatch;

    #[test]
    fn impls() {
        let f = format::Format::default().with_timer(time::Uptime::default());
        let subscriber = FmtSubscriber::builder().on_event(f).finish();
        let _dispatch = Dispatch::new(subscriber);

        let f = format::Format::default();
        let subscriber = FmtSubscriber::builder().on_event(f).finish();
        let _dispatch = Dispatch::new(subscriber);

        let f = format::Format::default().compact();
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
        assert!(dispatch.downcast_ref::<format::NewRecorder>().is_some());
        assert!(dispatch.downcast_ref::<filter::EnvFilter>().is_some());
        assert!(dispatch.downcast_ref::<format::Format>().is_some())
    }
}
