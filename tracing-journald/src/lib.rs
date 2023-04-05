//! # tracing-journald
//!
//! Support for logging [`tracing`] events natively to [journald],
//! preserving structured information.
//!
//! ## Overview
//!
//! [`tracing`] is a framework for instrumenting Rust programs to collect
//! scoped, structured, and async-aware diagnostics. `tracing-journald` provides a
//! [`tracing-subscriber::Layer`] implementation for logging `tracing` spans
//! and events to [`systemd-journald`][journald], on Linux distributions that
//! use `systemd`.
//!
//! *Compiler support: [requires `rustc` 1.56+][msrv]*
//!
//! [msrv]: #supported-rust-versions
//! [`tracing`]: https://crates.io/crates/tracing
//! [journald]: https://www.freedesktop.org/software/systemd/man/systemd-journald.service.html
//!
//! ## Supported Rust Versions
//!
//! Tracing is built against the latest stable release. The minimum supported
//! version is 1.56. The current Tracing version is not guaranteed to build on
//! Rust versions earlier than the minimum supported version.
//!
//! Tracing follows the same compiler support policies as the rest of the Tokio
//! project. The current stable Rust compiler and the three most recent minor
//! versions before it will always be supported. For example, if the current
//! stable compiler version is 1.69, the minimum supported version will not be
//! increased past 1.66, three minor versions prior. Increasing the minimum
//! supported compiler version is not considered a semver breaking change as
//! long as doing so complies with this policy.
//!
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/logo-type.png",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]
#![cfg_attr(docsrs, deny(rustdoc::broken_intra_doc_links))]
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

#[cfg(target_os = "linux")]
mod memfd;
#[cfg(target_os = "linux")]
mod socket;

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
/// The standard journald `CODE_LINE` and `CODE_FILE` fields are automatically emitted. A `TARGET`
/// field is emitted containing the event's target.
///
/// For events recorded inside spans, an additional `SPAN_NAME` field is emitted with the name of
/// each of the event's parent spans.
///
/// User-defined fields other than the event `message` field have a prefix applied by default to
/// prevent collision with standard fields.
///
/// [journald conventions]: https://www.freedesktop.org/software/systemd/man/systemd.journal-fields.html
pub struct Layer {
    #[cfg(unix)]
    socket: UnixDatagram,
    field_prefix: Option<String>,
    syslog_identifier: String,
}

#[cfg(unix)]
const JOURNALD_PATH: &str = "/run/systemd/journal/socket";

impl Layer {
    /// Construct a journald layer
    ///
    /// Fails if the journald socket couldn't be opened. Returns a `NotFound` error unconditionally
    /// in non-Unix environments.
    pub fn new() -> io::Result<Self> {
        #[cfg(unix)]
        {
            let socket = UnixDatagram::unbound()?;
            let layer = Self {
                socket,
                field_prefix: Some("F".into()),
                syslog_identifier: std::env::current_exe()
                    .ok()
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    // If we fail to get the name of the current executable fall back to an empty string.
                    .unwrap_or_else(String::new),
            };
            // Check that we can talk to journald, by sending empty payload which journald discards.
            // However if the socket didn't exist or if none listened we'd get an error here.
            layer.send_payload(&[])?;
            Ok(layer)
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

    /// Sets the syslog identifier for this logger.
    ///
    /// The syslog identifier comes from the classic syslog interface (`openlog()`
    /// and `syslog()`) and tags log entries with a given identifier.
    /// Systemd exposes it in the `SYSLOG_IDENTIFIER` journal field, and allows
    /// filtering log messages by syslog identifier with `journalctl -t`.
    /// Unlike the unit (`journalctl -u`) this field is not trusted, i.e. applications
    /// can set it freely, and use it e.g. to further categorize log entries emitted under
    /// the same systemd unit or in the same process.  It also allows to filter for log
    /// entries of processes not started in their own unit.
    ///
    /// See [Journal Fields](https://www.freedesktop.org/software/systemd/man/systemd.journal-fields.html)
    /// and [journalctl](https://www.freedesktop.org/software/systemd/man/journalctl.html)
    /// for more information.
    ///
    /// Defaults to the file name of the executable of the current process, if any.
    pub fn with_syslog_identifier(mut self, identifier: String) -> Self {
        self.syslog_identifier = identifier;
        self
    }

    /// Returns the syslog identifier in use.
    pub fn syslog_identifier(&self) -> &str {
        &self.syslog_identifier
    }

    #[cfg(not(unix))]
    fn send_payload(&self, _opayload: &[u8]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "journald not supported on non-Unix",
        ))
    }

    #[cfg(unix)]
    fn send_payload(&self, payload: &[u8]) -> io::Result<usize> {
        self.socket
            .send_to(payload, JOURNALD_PATH)
            .or_else(|error| {
                if Some(libc::EMSGSIZE) == error.raw_os_error() {
                    self.send_large_payload(payload)
                } else {
                    Err(error)
                }
            })
    }

    #[cfg(all(unix, not(target_os = "linux")))]
    fn send_large_payload(&self, _payload: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Large payloads not supported on non-Linux OS",
        ))
    }

    /// Send large payloads to journald via a memfd.
    #[cfg(target_os = "linux")]
    fn send_large_payload(&self, payload: &[u8]) -> io::Result<usize> {
        // If the payload's too large for a single datagram, send it through a memfd, see
        // https://systemd.io/JOURNAL_NATIVE_PROTOCOL/
        use std::os::unix::prelude::AsRawFd;
        // Write the whole payload to a memfd
        let mut mem = memfd::create_sealable()?;
        mem.write_all(payload)?;
        // Fully seal the memfd to signal journald that its backing data won't resize anymore
        // and so is safe to mmap.
        memfd::seal_fully(mem.as_raw_fd())?;
        socket::send_one_fd_to(&self.socket, mem.as_raw_fd(), JOURNALD_PATH)
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
    fn on_new_span(&self, attrs: &Attributes, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("unknown span");
        let mut buf = Vec::with_capacity(256);

        writeln!(buf, "SPAN_NAME").unwrap();
        put_value(&mut buf, span.name().as_bytes());
        put_metadata(&mut buf, span.metadata(), Some("SPAN_"));

        attrs.record(&mut SpanVisitor {
            buf: &mut buf,
            field_prefix: self.field_prefix.as_deref(),
        });

        span.extensions_mut().insert(SpanFields(buf));
    }

    fn on_record(&self, id: &Id, values: &Record, ctx: Context<S>) {
        let span = ctx.span(id).expect("unknown span");
        let mut exts = span.extensions_mut();
        let buf = &mut exts.get_mut::<SpanFields>().expect("missing fields").0;
        values.record(&mut SpanVisitor {
            buf,
            field_prefix: self.field_prefix.as_deref(),
        });
    }

    fn on_event(&self, event: &Event, ctx: Context<S>) {
        let mut buf = Vec::with_capacity(256);

        // Record span fields
        for span in ctx
            .lookup_current()
            .into_iter()
            .flat_map(|span| span.scope().from_root())
        {
            let exts = span.extensions();
            let fields = exts.get::<SpanFields>().expect("missing fields");
            buf.extend_from_slice(&fields.0);
        }

        // Record event fields
        put_priority(&mut buf, event.metadata());
        put_metadata(&mut buf, event.metadata(), None);
        put_field_length_encoded(&mut buf, "SYSLOG_IDENTIFIER", |buf| {
            write!(buf, "{}", self.syslog_identifier).unwrap()
        });

        event.record(&mut EventVisitor::new(
            &mut buf,
            self.field_prefix.as_deref(),
        ));

        // At this point we can't handle the error anymore so just ignore it.
        let _ = self.send_payload(&buf);
    }
}

struct SpanFields(Vec<u8>);

struct SpanVisitor<'a> {
    buf: &'a mut Vec<u8>,
    field_prefix: Option<&'a str>,
}

impl SpanVisitor<'_> {
    fn put_span_prefix(&mut self) {
        if let Some(prefix) = self.field_prefix {
            self.buf.extend_from_slice(prefix.as_bytes());
            self.buf.push(b'_');
        }
    }
}

impl Visit for SpanVisitor<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.put_span_prefix();
        put_field_length_encoded(self.buf, field.name(), |buf| {
            buf.extend_from_slice(value.as_bytes())
        });
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.put_span_prefix();
        put_field_length_encoded(self.buf, field.name(), |buf| {
            write!(buf, "{:?}", value).unwrap()
        });
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

    fn put_prefix(&mut self, field: &Field) {
        if let Some(prefix) = self.prefix {
            if field.name() != "message" {
                // message maps to the standard MESSAGE field so don't prefix it
                self.buf.extend_from_slice(prefix.as_bytes());
                self.buf.push(b'_');
            }
        }
    }
}

impl Visit for EventVisitor<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.put_prefix(field);
        put_field_length_encoded(self.buf, field.name(), |buf| {
            buf.extend_from_slice(value.as_bytes())
        });
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.put_prefix(field);
        put_field_length_encoded(self.buf, field.name(), |buf| {
            write!(buf, "{:?}", value).unwrap()
        });
    }
}

fn put_priority(buf: &mut Vec<u8>, meta: &Metadata) {
    put_field_wellformed(
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

fn put_metadata(buf: &mut Vec<u8>, meta: &Metadata, prefix: Option<&str>) {
    if let Some(prefix) = prefix {
        write!(buf, "{}", prefix).unwrap();
    }
    put_field_wellformed(buf, "TARGET", meta.target().as_bytes());
    if let Some(file) = meta.file() {
        if let Some(prefix) = prefix {
            write!(buf, "{}", prefix).unwrap();
        }
        put_field_wellformed(buf, "CODE_FILE", file.as_bytes());
    }
    if let Some(line) = meta.line() {
        if let Some(prefix) = prefix {
            write!(buf, "{}", prefix).unwrap();
        }
        // Text format is safe as a line number can't possibly contain anything funny
        writeln!(buf, "CODE_LINE={}", line).unwrap();
    }
}

/// Append a sanitized and length-encoded field into `buf`.
///
/// Unlike `put_field_wellformed` this function handles arbitrary field names and values.
///
/// `name` denotes the field name. It gets sanitized before being appended to `buf`.
///
/// `write_value` is invoked with `buf` as argument to append the value data to `buf`.  It must
/// not delete from `buf`, but may append arbitrary data.  This function then determines the length
/// of the data written and adds it in the appropriate place in `buf`.
fn put_field_length_encoded(buf: &mut Vec<u8>, name: &str, write_value: impl FnOnce(&mut Vec<u8>)) {
    sanitize_name(name, buf);
    buf.push(b'\n');
    buf.extend_from_slice(&[0; 8]); // Length tag, to be populated
    let start = buf.len();
    write_value(buf);
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

/// Append arbitrary data with a well-formed name and value.
///
/// `value` must not contain an internal newline, because this function writes
/// `value` in the new-line separated format.
///
/// For a "newline-safe" variant, see `put_field_length_encoded`.
fn put_field_wellformed(buf: &mut Vec<u8>, name: &str, value: &[u8]) {
    buf.extend_from_slice(name.as_bytes());
    buf.push(b'\n');
    put_value(buf, value);
}

/// Write the value portion of a key-value pair, in newline separated format.
///
/// `value` must not contain an internal newline.
///
/// For a "newline-safe" variant, see `put_field_length_encoded`.
fn put_value(buf: &mut Vec<u8>, value: &[u8]) {
    buf.extend_from_slice(&(value.len() as u64).to_le_bytes());
    buf.extend_from_slice(value);
    buf.push(b'\n');
}
