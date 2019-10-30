use crate::{
    fmt::{format, FormatEvent, FormatFields, MakeWriter},
    layer::{Context, Layer},
    registry::{Extensions, LookupMetadata, LookupSpan, Registry, SpanData, SpanRef},
};
use ansi_term::{Color, Style};
use humantime;
use std::{
    any::type_name,
    fmt::{self, Write as _},
    io::{self, Write},
    marker::PhantomData,
    time::SystemTime,
};
use tracing_core::{
    field::{Field, Visit},
    span::{Attributes, Id},
    Event, Level, Subscriber,
};

pub struct FmtLayer<
    S = Registry,
    N = format::DefaultFields,
    E = format::Format<format::Full>,
    W = fn() -> io::Stdout,
> {
    is_interested: Box<dyn Fn(&Event<'_>) -> bool + Send + Sync + 'static>,
    inner: PhantomData<S>,
    make_writer: W,
    fmt_fields: N,
    fmt_event: E,
}

pub struct FmtLayerBuilder<
    S = Registry,
    N = format::DefaultFields,
    E = format::Format<format::Full>,
    W = fn() -> io::Stdout,
> {
    fmt_fields: N,
    fmt_event: E,
    make_writer: W,
    is_interested: Box<dyn Fn(&Event<'_>) -> bool + Send + Sync + 'static>,
    inner: PhantomData<S>,
}

impl FmtLayer {
    pub fn builder() -> FmtLayerBuilder {
        FmtLayerBuilder::default()
    }
}

impl<S, N, E, W> FmtLayerBuilder<S, N, E, W>
where
    S: Subscriber + for<'a> LookupSpan<'a> + LookupMetadata,
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<S, N> + 'static,
    W: MakeWriter + 'static,
{
    pub fn with_interest<F>(self, f: F) -> Self
    where
        F: Fn(&Event<'_>) -> bool + Send + Sync + 'static,
    {
        Self {
            is_interested: Box::new(f),
            ..self
        }
    }
}

// this needs to be a seperate impl block because we're re-assigning the the W2 (make_writer)
// type paramater from the default.
impl<S, N, E, W> FmtLayerBuilder<S, N, E, W> {
    pub fn with_writer<W2>(self, make_writer: W2) -> FmtLayerBuilder<S, N, E, W2>
    where
        W2: MakeWriter + 'static,
    {
        FmtLayerBuilder {
            fmt_fields: self.fmt_fields,
            fmt_event: self.fmt_event,
            is_interested: self.is_interested,
            inner: self.inner,
            make_writer,
        }
    }
}

impl<S, N, E, W> FmtLayerBuilder<S, N, E, W>
where
    S: Subscriber + for<'a> LookupSpan<'a> + LookupMetadata,
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<S, N> + 'static,
    W: MakeWriter + 'static,
{
    pub fn build(self) -> FmtLayer<S, N, E, W> {
        FmtLayer {
            is_interested: self.is_interested,
            inner: self.inner,
            make_writer: self.make_writer,
            fmt_fields: self.fmt_fields,
            fmt_event: self.fmt_event,
        }
    }
}

impl Default for FmtLayerBuilder {
    fn default() -> Self {
        Self {
            is_interested: Box::new(|_| true),
            inner: PhantomData,
            fmt_fields: format::DefaultFields::default(),
            fmt_event: format::Format::default(),
            make_writer: io::stdout,
        }
    }
}

fn name_of<T>(t: T) -> &'static str {
    type_name::<T>()
}

// === impl Formatter ===

impl<S, N, E, W> Layer<S> for FmtLayer<S, N, E, W>
where
    S: Subscriber + for<'a> LookupSpan<'a> + LookupMetadata,
    N: for<'writer> FormatFields<'writer> + 'static,
    E: FormatEvent<S, N> + 'static,
    W: MakeWriter + 'static,
{
    fn on_close(&self, id: Id, _: Context<'_, S>) {
        // dbg!(id);
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut buf = String::new();
        let ctx = FmtContext {
            ctx: ctx,
            fmt_fields: self.fmt_fields,
        };
        if self.fmt_event.format_event(ctx, &mut buf, event).is_ok() {
            let mut writer = self.make_writer.make_writer();
            let _ = io::Write::write_all(&mut writer, buf.as_bytes());
        }
    }
}

pub(crate) struct FmtContext<'a, S, N>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup> + LookupMetadata,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    ctx: Context<'a, S>,
    fmt_fields: &'a N,
}

struct EventVisitor {
    comma: bool,
    buf: String,
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        write!(
            &mut self.buf,
            "{comma} ",
            comma = if self.comma { "," } else { "" },
        )
        .unwrap();
        let name = field.name();
        if name == "message" {
            write!(
                &mut self.buf,
                "{}",
                Style::new().bold().paint(format!("{:?}", value))
            )
            .unwrap();
            self.comma = true;
        } else {
            write!(self.buf, "{}: {:?}", Style::new().bold().paint(name), value).unwrap();
            self.comma = true;
        }
    }
}

struct ColorLevel<'a>(&'a Level);

impl<'a> fmt::Display for ColorLevel<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            &Level::TRACE => Color::Purple.paint("TRACE"),
            &Level::DEBUG => Color::Blue.paint("DEBUG"),
            &Level::INFO => Color::Green.paint("INFO "),
            &Level::WARN => Color::Yellow.paint("WARN "),
            &Level::ERROR => Color::Red.paint("ERROR"),
        }
        .fmt(f)
    }
}
