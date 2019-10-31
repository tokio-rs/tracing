use crate::{
    field::RecordFields,
    fmt::{
        format::{self, FmtLevel, FullCtx},
        time::{self, FormatTime, SystemTime},
        FormatEvent, FormatFields, MakeWriter,
    },
    layer::{Context, Layer},
    registry::{LookupMetadata, LookupSpan, Registry, SpanRef},
};
use ansi_term::{Color, Style};
use std::{
    fmt::{self, Write},
    io,
    marker::PhantomData,
};
use tracing_core::{
    field::{Field, Visit},
    span::Id,
    Event, Level, Subscriber,
};
use tracing_log::NormalizeEvent;

pub struct FmtLayer<S = Registry, N = format::DefaultFields, W = fn() -> io::Stdout> {
    is_interested: Box<dyn Fn(&Event<'_>) -> bool + Send + Sync + 'static>,
    inner: PhantomData<S>,
    make_writer: W,
    fmt_fields: N,
    fmt_event: format::Format<format::Full>,
}

pub struct FmtLayerBuilder<S = Registry, N = format::DefaultFields, W = fn() -> io::Stdout> {
    fmt_fields: N,
    make_writer: W,
    is_interested: Box<dyn Fn(&Event<'_>) -> bool + Send + Sync + 'static>,
    inner: PhantomData<S>,
}

impl FmtLayer {
    pub fn builder() -> FmtLayerBuilder {
        FmtLayerBuilder::default()
    }
}

impl<S, N, W> FmtLayerBuilder<S, N, W>
where
    S: Subscriber + for<'a> LookupSpan<'a> + LookupMetadata,
    N: for<'writer> FormatFields<'writer> + 'static,
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
impl<S, N, W> FmtLayerBuilder<S, N, W>
where
    S: Subscriber + for<'a> LookupSpan<'a> + LookupMetadata,
    N: for<'writer> FormatFields<'writer> + 'static,
    W: MakeWriter + 'static,
{
    pub fn with_writer<W2>(self, make_writer: W2) -> FmtLayerBuilder<S, N, W2>
    where
        W2: MakeWriter + 'static,
    {
        FmtLayerBuilder {
            fmt_fields: self.fmt_fields,
            is_interested: self.is_interested,
            inner: self.inner,
            make_writer,
        }
    }
}

impl<S, N, W> FmtLayerBuilder<S, N, W>
where
    S: Subscriber + for<'a> LookupSpan<'a> + LookupMetadata,
    N: for<'writer> FormatFields<'writer> + 'static,
    W: MakeWriter + 'static,
{
    pub fn build(self) -> FmtLayer<S, N, W> {
        let fmt = format::Format::default();
        FmtLayer {
            is_interested: self.is_interested,
            inner: self.inner,
            make_writer: self.make_writer,
            fmt_fields: self.fmt_fields,
            fmt_event: fmt,
        }
    }
}

impl Default for FmtLayerBuilder {
    fn default() -> Self {
        Self {
            is_interested: Box::new(|_| true),
            inner: PhantomData,
            fmt_fields: format::DefaultFields::default(),
            make_writer: io::stdout,
        }
    }
}

impl<S, N> FormatEvent<S, N> for format::Format<format::Full>
where
    S: Subscriber + for<'a> LookupSpan<'a> + LookupMetadata,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        writer: &mut dyn fmt::Write,
        event: &Event<'_>,
    ) -> fmt::Result {
        #[cfg(feature = "tracing-log")]
        let normalized_meta = event.normalized_metadata();
        #[cfg(feature = "tracing-log")]
        let meta = normalized_meta.as_ref().unwrap_or_else(|| event.metadata());
        #[cfg(not(feature = "tracing-log"))]
        let meta = event.metadata();
        #[cfg(feature = "ansi")]
        time::write(&self.timer, writer, self.ansi)?;
        #[cfg(not(feature = "ansi"))]
        time::write(&self.timer, writer)?;

        let (fmt_level, full_ctx) = {
            #[cfg(feature = "ansi")]
            {
                (
                    FmtLevel::new(meta.level(), self.ansi),
                    FullCtx::new(ctx, self.ansi),
                )
            }
            #[cfg(not(feature = "ansi"))]
            {
                (FmtLevel::new(meta.level()), FullCtx::new(&ctx))
            }
        };

        write!(
            writer,
            "{} {}{}: ",
            fmt_level,
            full_ctx,
            if self.display_target {
                meta.target()
            } else {
                ""
            }
        )?;
        ctx.format_fields(writer, event)?;
        writeln!(writer)
    }
}

// === impl Formatter ===

impl<S, N, W> Layer<S> for FmtLayer<S, N, W>
where
    S: Subscriber + for<'a> LookupSpan<'a> + LookupMetadata,
    N: for<'writer> FormatFields<'writer> + 'static,
    W: MakeWriter + 'static,
{
    fn on_close(&self, id: Id, _: Context<'_, S>) {
        // dbg!(id);
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut buf = String::new();
        let ctx = FmtContext {
            ctx,
            fmt_fields: &self.fmt_fields,
        };
        if self.fmt_event.format_event(&ctx, &mut buf, event).is_ok() {
            let mut writer = self.make_writer.make_writer();
            let _ = io::Write::write_all(&mut writer, buf.as_bytes());
        }
    }
}

pub struct FmtContext<'a, S, N>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup> + LookupMetadata,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    pub ctx: Context<'a, S>,
    pub fmt_fields: &'a N,
}

impl<'a, S, N> FormatFields<'a> for FmtContext<'a, S, N>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup> + LookupMetadata,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_fields<R: RecordFields>(
        &self,
        writer: &'a mut dyn fmt::Write,
        fields: R,
    ) -> fmt::Result {
        self.fmt_fields.format_fields(writer, fields)
    }
}

impl<'a, S, N> FmtContext<'a, S, N>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup> + LookupMetadata,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    pub fn visit_spans<E, F>(&self, f: F) -> Result<(), E>
    where
        F: FnMut(&Id) -> Result<(), E>,
    {
        Ok(())
    }
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
