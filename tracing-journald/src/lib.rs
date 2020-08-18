//! # tracing-journald
//!
//! Support for logging [`tracing`][tracing] events natively to [journald],
//! preserving structured information.
//!
//! ## Overview
//!
//! [`tracing`] is a framework for instrumenting Rust programs to collect
//! scoped, structured, and async-aware diagnostics. `tracing-journald` provides a
//! [`tracing-subscriber::Layer`][layer] implementation for logging `tracing` spans
//! and events to [`systemd-journald`][journald], on Linux distributions that
//! use `systemd`.
//!  
//! *Compiler support: [requires `rustc` 1.40+][msrv]*
//!
//! [msrv]: #supported-rust-versions
//! [`tracing`]: https://crates.io/crates/tracing
//! [layer]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html
//! [journald]: https://www.freedesktop.org/software/systemd/man/systemd-journald.service.html
//!
//! ## Supported Rust Versions
//!
//! Tracing is built against the latest stable release. The minimum supported
//! version is 1.40. The current Tracing version is not guaranteed to build on
//! Rust versions earlier than the minimum supported version.
//!
//! Tracing follows the same compiler support policies as the rest of the Tokio
//! project. The current stable Rust compiler and the three most recent minor
//! versions before it will always be supported. For example, if the current
//! stable compiler version is 1.45, the minimum supported version will not be
//! increased past 1.42, three minor versions prior. Increasing the minimum
//! supported compiler version is not considered a semver breaking change as
//! long as doing so complies with this policy.
//!
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/logo.svg",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]
#[cfg(unix)]
use std::os::unix::net::UnixDatagram;
use std::{fmt, io, io::Write};

use tracing_core::{
    event::Event,
    field::Visit,
    span::{Attributes, Id, Record},
    Field, Level, Metadata, Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan};

/// Sends events and their fields to journald
///
/// [journald conventions] for structured field names differ from typical tracing idioms, and journald
/// discards fields which violate its conventions. Hence, this layer automatically sanitizes field
/// names by translating `.`s into `_`s, stripping leading `_`s and non-ascii-alphanumeric
/// characters other than `_`, and upcasing.
///
/// Levels are mapped losslessly to journald `PRIORITY` values as follows:
///
/// - `ERROR` => Error (3)
/// - `WARN` => Warning (4)
/// - `INFO` => Notice (5)
/// - `DEBUG` => Informational (6)
/// - `TRACE` => Debug (7)
///
/// Note that the naming scheme differs slightly for the latter half.
///
/// The standard journald `CODE_LINE` and `CODE_FILE` fields are automatically emitted. A `TARGET`
/// field is emitted containing the event's target. Enclosing spans are numbered counting up from
/// the root, and their fields and metadata are included in fields prefixed by `Sn_` where `n` is
/// that number.
///
/// User-defined fields other than the event `message` field have a prefix applied by default to
/// prevent collision with standard fields.
///
/// [journald conventions]: https://www.freedesktop.org/software/systemd/man/systemd.journal-fields.html
pub struct Layer {
    #[cfg(unix)]
    socket: UnixDatagram,
    field_prefix: Option<String>,
}

impl Layer {
    /// Construct a journald layer
    ///
    /// Fails if the journald socket couldn't be opened. Returns a `NotFound` error unconditionally
    /// in non-Unix environments.
    pub fn new() -> io::Result<Self> {
        #[cfg(unix)]
        {
            let socket = UnixDatagram::unbound()?;
            socket.connect("/run/systemd/journal/socket")?;
            Ok(Self {
                socket,
                field_prefix: Some("F".into()),
            })
        }
        #[cfg(not(unix))]
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "journald does not exist in this environment",
        ))
    }

    /// Sets the prefix to apply to names of user-defined fields other than the event `message`
    /// field. Defaults to `Some("F")`.
    pub fn with_field_prefix(mut self, x: Option<String>) -> Self {
        self.field_prefix = x;
        self
    }
}

/// Construct a journald layer
///
/// Fails if the journald socket couldn't be opened.
pub fn layer() -> io::Result<Layer> {
    Layer::new()
}

impl<S> tracing_subscriber::Layer<S> for Layer
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn new_span(&self, attrs: &Attributes, id: &Id, ctx: Context<S>) {
        let span = ctx.span(id).expect("unknown span");
        let mut buf = Vec::with_capacity(256);

        let depth = span.parents().count();

        writeln!(buf, "S{}_NAME", depth).unwrap();
        put_value(&mut buf, span.name().as_bytes());
        put_metadata(&mut buf, span.metadata(), Some(depth));

        attrs.record(&mut SpanVisitor {
            buf: &mut buf,
            depth,
            prefix: self.field_prefix.as_ref().map(|x| &x[..]),
        });

        span.extensions_mut().insert(SpanFields(buf));
    }

    fn on_record(&self, id: &Id, values: &Record, ctx: Context<S>) {
        let span = ctx.span(id).expect("unknown span");
        let depth = span.parents().count();
        let mut exts = span.extensions_mut();
        let buf = &mut exts.get_mut::<SpanFields>().expect("missing fields").0;
        values.record(&mut SpanVisitor {
            buf,
            depth,
            prefix: self.field_prefix.as_ref().map(|x| &x[..]),
        });
    }

    fn on_event(&self, event: &Event, ctx: Context<S>) {
        let mut buf = Vec::with_capacity(256);

        // Record span fields
        for span in ctx.scope() {
            let exts = span.extensions();
            let fields = exts.get::<SpanFields>().expect("missing fields");
            buf.extend_from_slice(&fields.0);
        }

        // Record event fields
        put_metadata(&mut buf, event.metadata(), None);
        event.record(&mut EventVisitor::new(
            &mut buf,
            self.field_prefix.as_ref().map(|x| &x[..]),
        ));

        // What could we possibly do on error?
        #[cfg(unix)]
        let _ = self.socket.send(&buf);
    }
}

struct SpanFields(Vec<u8>);

struct SpanVisitor<'a> {
    buf: &'a mut Vec<u8>,
    depth: usize,
    prefix: Option<&'a str>,
}

impl Visit for SpanVisitor<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        write!(self.buf, "S{}", self.depth).unwrap();
        if let Some(prefix) = self.prefix {
            self.buf.extend_from_slice(prefix.as_bytes());
        }
        self.buf.push(b'_');
        put_debug(self.buf, field.name(), value);
    }
}

/// Helper for generating the journal export format, which is consumed by journald:
/// https://www.freedesktop.org/wiki/Software/systemd/export/
struct EventVisitor<'a> {
    buf: &'a mut Vec<u8>,
    prefix: Option<&'a str>,
}

impl<'a> EventVisitor<'a> {
    fn new(buf: &'a mut Vec<u8>, prefix: Option<&'a str>) -> Self {
        Self { buf, prefix }
    }
}

impl Visit for EventVisitor<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if let Some(prefix) = self.prefix {
            if field.name() != "message" {
                self.buf.extend_from_slice(prefix.as_bytes());
                self.buf.push(b'_');
            }
        }
        put_debug(self.buf, field.name(), value);
    }
}

fn put_metadata(buf: &mut Vec<u8>, meta: &Metadata, span: Option<usize>) {
    if span.is_none() {
        put_field(
            buf,
            "PRIORITY",
            match *meta.level() {
                Level::ERROR => b"3",
                Level::WARN => b"4",
                Level::INFO => b"5",
                Level::DEBUG => b"6",
                Level::TRACE => b"7",
            },
        );
    }
    if let Some(n) = span {
        write!(buf, "S{}_", n).unwrap();
    }
    put_field(buf, "TARGET", meta.target().as_bytes());
    if let Some(file) = meta.file() {
        if let Some(n) = span {
            write!(buf, "S{}_", n).unwrap();
        }
        put_field(buf, "CODE_FILE", file.as_bytes());
    }
    if let Some(line) = meta.line() {
        if let Some(n) = span {
            write!(buf, "S{}_", n).unwrap();
        }
        // Text format is safe as a line number can't possibly contain anything funny
        writeln!(buf, "CODE_LINE={}", line).unwrap();
    }
}

fn put_debug(buf: &mut Vec<u8>, name: &str, value: &dyn fmt::Debug) {
    sanitize_name(name, buf);
    buf.push(b'\n');
    buf.extend_from_slice(&[0; 8]); // Length tag, to be populated
    let start = buf.len();
    write!(buf, "{:?}", value).unwrap();
    let end = buf.len();
    buf[start - 8..start].copy_from_slice(&((end - start) as u64).to_le_bytes());
    buf.push(b'\n');
}

/// Mangle a name into journald-compliant form
fn sanitize_name(name: &str, buf: &mut Vec<u8>) {
    buf.extend(
        name.bytes()
            .map(|c| if c == b'.' { b'_' } else { c })
            .skip_while(|&c| c == b'_')
            .filter(|&c| c == b'_' || char::from(c).is_ascii_alphanumeric())
            .map(|c| char::from(c).to_ascii_uppercase() as u8),
    );
}

/// Append arbitrary data with a well-formed name
fn put_field(buf: &mut Vec<u8>, name: &str, value: &[u8]) {
    buf.extend_from_slice(name.as_bytes());
    buf.push(b'\n');
    put_value(buf, value);
}

/// Write the value portion of a key-value pair
fn put_value(buf: &mut Vec<u8>, value: &[u8]) {
    buf.extend_from_slice(&(value.len() as u64).to_le_bytes());
    buf.extend_from_slice(value);
    buf.push(b'\n');
}
