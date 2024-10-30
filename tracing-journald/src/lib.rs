//! # tracing-journald
//!
//! Support for logging [`tracing`] events natively to [journald],
//! preserving structured information.
//!
//! ## Overview
//!
//! [`tracing`] is a framework for instrumenting Rust programs to collect
//! scoped, structured, and async-aware diagnostics. `tracing-journald` provides a
//! [`tracing-subscriber::Subscriber`][subscriber] implementation for logging `tracing` spans
//! and events to [`systemd-journald`][journald], on Linux distributions that
//! use `systemd`.
//!
//! *Compiler support: [requires `rustc` 1.63+][msrv]*
//!
//! [msrv]: #supported-rust-versions
//! [`tracing`]: https://crates.io/crates/tracing
//! [subscriber]: tracing_subscriber::subscribe::Subscribe
//! [journald]: https://www.freedesktop.org/software/systemd/man/systemd-journald.service.html
//!
//! ## Supported Rust Versions
//!
//! Tracing is built against the latest stable release. The minimum supported
//! version is 1.63. The current Tracing version is not guaranteed to build on
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
    html_favicon_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/favicon.ico",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]

#[cfg(unix)]
use std::os::unix::net::UnixDatagram;
use std::{fmt, io, io::Write};

use tracing_core::{
    event::Event,
    field::Visit,
    span::{Attributes, Id, Record},
    Collect, Field, Level, Metadata,
};
use tracing_subscriber::{registry::LookupSpan, subscribe::Context};

#[cfg(target_os = "linux")]
mod memfd;
#[cfg(target_os = "linux")]
mod socket;

/// Sends events and their fields to journald
///
/// [journald conventions] for structured field names differ from typical tracing idioms, and journald
/// discards fields which violate its conventions. Hence, this subscriber automatically sanitizes field
/// names by translating `.`s into `_`s, stripping leading `_`s and non-ascii-alphanumeric
/// characters other than `_`, and upcasing.
///
/// By default, levels are mapped losslessly to journald `PRIORITY` values as follows:
///
/// - `ERROR` => Error (3)
/// - `WARN` => Warning (4)
/// - `INFO` => Notice (5)
/// - `DEBUG` => Informational (6)
/// - `TRACE` => Debug (7)
///
/// These mappings can be changed with [`Subscriber::with_priority_mappings`].
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
pub struct Subscriber {
    #[cfg(unix)]
    socket: UnixDatagram,
    field_prefix: Option<String>,
    syslog_identifier: String,
    additional_fields: Vec<u8>,
    priority_mappings: PriorityMappings,
}

#[cfg(unix)]
const JOURNALD_PATH: &str = "/run/systemd/journal/socket";

impl Subscriber {
    /// Construct a journald subscriber
    ///
    /// Fails if the journald socket couldn't be opened. Returns a `NotFound` error unconditionally
    /// in non-Unix environments.
    pub fn new() -> io::Result<Self> {
        #[cfg(unix)]
        {
            let socket = UnixDatagram::unbound()?;
            let sub = Self {
                socket,
                field_prefix: Some("F".into()),
                syslog_identifier: std::env::current_exe()
                    .ok()
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    // If we fail to get the name of the current executable fall back to an empty string.
                    .unwrap_or_default(),
                additional_fields: Vec::new(),
                priority_mappings: PriorityMappings::new(),
            };
            // Check that we can talk to journald, by sending empty payload which journald discards.
            // However if the socket didn't exist or if none listened we'd get an error here.
            sub.send_payload(&[])?;
            Ok(sub)
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

    /// Sets how [`tracing_core::Level`]s are mapped to [journald priorities](Priority).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tracing_journald::{Priority, PriorityMappings};
    /// use tracing_subscriber::prelude::*;
    /// use tracing::error;
    ///
    /// let registry = tracing_subscriber::registry();
    /// match tracing_journald::subscriber() {
    ///     Ok(subscriber) => {
    ///         registry.with(
    ///             subscriber
    ///                 // We can tweak the mappings between the trace level and
    ///                 // the journal priorities.
    ///                 .with_priority_mappings(PriorityMappings {
    ///                     info: Priority::Informational,
    ///                     ..PriorityMappings::new()
    ///                 }),
    ///         );
    ///     }
    ///     // journald is typically available on Linux systems, but nowhere else. Portable software
    ///     // should handle its absence gracefully.
    ///     Err(e) => {
    ///         registry.init();
    ///         error!("couldn't connect to journald: {}", e);
    ///     }
    /// }
    /// ```
    pub fn with_priority_mappings(mut self, mappings: PriorityMappings) -> Self {
        self.priority_mappings = mappings;
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

    /// Adds fields that will get be passed to journald with every log entry.
    ///
    /// The input values of this function are interpreted as `(field, value)` pairs.
    ///
    /// This can for example be used to configure the syslog facility.
    /// See [Journal Fields](https://www.freedesktop.org/software/systemd/man/systemd.journal-fields.html)
    /// and [journalctl](https://www.freedesktop.org/software/systemd/man/journalctl.html)
    /// for more information.
    ///
    /// Fields specified using this method will be added to the journald
    /// message alongside fields generated from the event's fields, its
    /// metadata, and the span context. If the name of a field provided using
    /// this method is the same as the name of a field generated by the
    /// subscriber, both fields will be sent to journald.
    ///
    /// ```no_run
    /// # use tracing_journald::Subscriber;
    /// let sub = Subscriber::new()
    ///     .unwrap()
    ///     .with_custom_fields([("SYSLOG_FACILITY", "17")]);
    /// ```
    ///
    pub fn with_custom_fields<T: AsRef<str>, U: AsRef<[u8]>>(
        mut self,
        fields: impl IntoIterator<Item = (T, U)>,
    ) -> Self {
        for (name, value) in fields {
            put_field_length_encoded(&mut self.additional_fields, name.as_ref(), |buf| {
                buf.extend_from_slice(value.as_ref())
            })
        }
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

    fn put_priority(&self, buf: &mut Vec<u8>, meta: &Metadata) {
        put_field_wellformed(
            buf,
            "PRIORITY",
            &[match *meta.level() {
                Level::ERROR => self.priority_mappings.error as u8,
                Level::WARN => self.priority_mappings.warn as u8,
                Level::INFO => self.priority_mappings.info as u8,
                Level::DEBUG => self.priority_mappings.debug as u8,
                Level::TRACE => self.priority_mappings.trace as u8,
            }],
        );
    }
}

/// Construct a journald subscriber
///
/// Fails if the journald socket couldn't be opened.
pub fn subscriber() -> io::Result<Subscriber> {
    Subscriber::new()
}

impl<C> tracing_subscriber::Subscribe<C> for Subscriber
where
    C: Collect + for<'span> LookupSpan<'span>,
{
    fn on_new_span(&self, attrs: &Attributes, id: &Id, ctx: Context<C>) {
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

    fn on_record(&self, id: &Id, values: &Record, ctx: Context<C>) {
        let span = ctx.span(id).expect("unknown span");
        let mut exts = span.extensions_mut();
        let buf = &mut exts.get_mut::<SpanFields>().expect("missing fields").0;
        values.record(&mut SpanVisitor {
            buf,
            field_prefix: self.field_prefix.as_deref(),
        });
    }

    fn on_event(&self, event: &Event, ctx: Context<C>) {
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
        self.put_priority(&mut buf, event.metadata());
        put_metadata(&mut buf, event.metadata(), None);
        put_field_length_encoded(&mut buf, "SYSLOG_IDENTIFIER", |buf| {
            write!(buf, "{}", self.syslog_identifier).unwrap()
        });
        buf.extend_from_slice(&self.additional_fields);

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

/// A priority (called "severity code" by syslog) is used to mark the
/// importance of a message.
///
/// Descriptions and examples are taken from the [Arch Linux wiki].
/// Priorities are also documented in the
/// [section 6.2.1 of the Syslog protocol RFC][syslog].
///
/// [Arch Linux wiki]: https://wiki.archlinux.org/title/Systemd/Journal#Priority_level
/// [syslog]: https://www.rfc-editor.org/rfc/rfc5424#section-6.2.1
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[repr(u8)]
pub enum Priority {
    /// System is unusable.
    ///
    /// Examples:
    ///
    /// - severe Kernel BUG
    /// - systemd dumped core
    ///
    /// This level should not be used by applications.
    Emergency = b'0',
    /// Should be corrected immediately.
    ///
    /// Examples:
    ///
    /// - Vital subsystem goes out of work, data loss:
    /// - `kernel: BUG: unable to handle kernel paging request at ffffc90403238ffc`
    Alert = b'1',
    /// Critical conditions
    ///
    /// Examples:
    ///
    /// - Crashe, coredumps
    /// - `systemd-coredump[25319]: Process 25310 (plugin-container) of user 1000 dumped core`
    Critical = b'2',
    /// Error conditions
    ///
    /// Examples:
    ///
    /// - Not severe error reported
    /// - `kernel: usb 1-3: 3:1: cannot get freq at ep 0x84, systemd[1]: Failed unmounting /var`
    /// - `libvirtd[1720]: internal error: Failed to initialize a valid firewall backend`
    Error = b'3',
    /// May indicate that an error will occur if action is not taken.
    ///
    /// Examples:
    ///
    /// - a non-root file system has only 1GB free
    /// - `org.freedesktop. Notifications[1860]: (process:5999): Gtk-WARNING **: Locale not supported by C library. Using the fallback 'C' locale`
    Warning = b'4',
    /// Events that are unusual, but not error conditions.
    ///
    /// Examples:
    ///
    /// - `systemd[1]: var.mount: Directory /var to mount over is not empty, mounting anyway`
    /// - `gcr-prompter[4997]: Gtk: GtkDialog mapped without a transient parent. This is discouraged`
    Notice = b'5',
    /// Normal operational messages that require no action.
    ///
    /// Example: `lvm[585]: 7 logical volume(s) in volume group "archvg" now active`
    Informational = b'6',
    /// Information useful to developers for debugging the
    /// application.
    ///
    /// Example: `kdeinit5[1900]: powerdevil: Scheduling inhibition from ":1.14" "firefox" with cookie 13 and reason "screen"`
    Debug = b'7',
}

/// Mappings from tracing [`Level`]s to journald [priorities].
///
/// [priorities]: Priority
#[derive(Debug, Clone)]
pub struct PriorityMappings {
    /// Priority mapped to the `ERROR` level
    pub error: Priority,
    /// Priority mapped to the `WARN` level
    pub warn: Priority,
    /// Priority mapped to the `INFO` level
    pub info: Priority,
    /// Priority mapped to the `DEBUG` level
    pub debug: Priority,
    /// Priority mapped to the `TRACE` level
    pub trace: Priority,
}

impl PriorityMappings {
    /// Returns the default priority mappings:
    ///
    /// - [`tracing::Level::ERROR`]: [`Priority::Error`] (3)
    /// - [`tracing::Level::WARN`]: [`Priority::Warning`] (4)
    /// - [`tracing::Level::INFO`]: [`Priority::Notice`] (5)
    /// - [`tracing::Level::DEBUG`]: [`Priority::Informational`] (6)
    /// - [`tracing::Level::TRACE`]: [`Priority::Debug`] (7)
    pub fn new() -> PriorityMappings {
        Self {
            error: Priority::Error,
            warn: Priority::Warning,
            info: Priority::Notice,
            debug: Priority::Informational,
            trace: Priority::Debug,
        }
    }
}

impl Default for PriorityMappings {
    fn default() -> Self {
        Self::new()
    }
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
