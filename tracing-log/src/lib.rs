//! Adapters for connecting unstructured log records from the `log` crate into
//! the `tracing` ecosystem.
//!
//! # Overview
//!
//! [`tracing`] is a framework for instrumenting Rust programs with context-aware,
//! structured, event-based diagnostic information. This crate provides
//! compatibility layers for using `tracing` alongside the logging facade provided
//! by the [`log`] crate.
//!
//! This crate provides:
//!
//! - [`AsTrace`] and [`AsLog`] traits for converting between `tracing` and `log` types.
//! - [`LogTracer`], a [`log::Log`] implementation that consumes [`log::Record`]s
//!   and outputs them as [`tracing::Event`].
//! - An [`env_logger`] module, with helpers for using the [`env_logger` crate]
//!   with `tracing` (optional, enabled by the `env-logger` feature).
//!
//! *Compiler support: [requires `rustc` 1.49+][msrv]*
//!
//! [msrv]: #supported-rust-versions
//!
//! # Usage
//!
//! ## Convert log records to tracing `Event`s
//!
//! To convert [`log::Record`]s as [`tracing::Event`]s, set `LogTracer` as the default
//! logger by calling its [`init`] or [`init_with_filter`] methods.
//!
//! ```rust
//! # use std::error::Error;
//! use tracing_log::LogTracer;
//! use log;
//!
//! # fn main() -> Result<(), Box<dyn Error>> {
//! LogTracer::init()?;
//!
//! // will be available for Subscribers as a tracing Event
//! log::trace!("an example trace log");
//! # Ok(())
//! # }
//! ```
//!
//! This conversion does not convert unstructured data in log records (such as
//! values passed as format arguments to the `log!` macro) to structured
//! `tracing` fields. However, it *does* attach these new events to to the
//! span that was currently executing when the record was logged. This is the
//! primary use-case for this library: making it possible to locate the log
//! records emitted by dependencies which use `log` within the context of a
//! trace.
//!
//! ## Convert tracing `Event`s to logs
//!
//! Enabling the ["log" and "log-always" feature flags][flags] on the `tracing`
//! crate will cause all `tracing` spans and events to emit `log::Record`s as
//! they occur.
//!
//! ## Caution: Mixing both conversions
//!
//! Note that logger implementations that convert log records to trace events
//! should not be used with `Collector`s that convert trace events _back_ into
//! log records, as doing so will result in the event recursing between the
//! collector and the logger forever (or, in real life, probably overflowing
//! the call stack).
//!
//! If the logging of trace events generated from log records produced by the
//! `log` crate is desired, either the `log` crate should not be used to
//! implement this logging, or an additional subscriber of filtering will be
//! required to avoid infinitely converting between `Event` and `log::Record`.
//!
//! # Feature Flags
//! * `log-tracer`: enables the `LogTracer` type (on by default)
//! * `env_logger`: enables the `env_logger` module, with helpers for working
//!   with the [`env_logger` crate].
//!
//! ## Supported Rust Versions
//!
//! Tracing is built against the latest stable release. The minimum supported
//! version is 1.49. The current Tracing version is not guaranteed to build on
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
//! [`init`]: LogTracer::init()
//! [`init_with_filter`]: LogTracer::init_with_filter()
//! [`tracing`]: https://crates.io/crates/tracing
//! [`log`]: https://crates.io/crates/log
//! [`env_logger` crate]: https://crates.io/crates/env-logger
//! [`tracing::Collector`]: tracing::Collect
//! [`tracing::Event`]: tracing_core::Event
//! [`Collect`]: tracing::Collect
//! [flags]: https://docs.rs/tracing/latest/tracing/#crate-feature-flags
#![doc(html_root_url = "https://docs.rs/tracing-log/0.1.1")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/logo-type.png",
    html_favicon_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/favicon.ico",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub,
    bad_style,
    const_err,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_in_public,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true
)]
use lazy_static::lazy_static;

use std::{convert::TryInto, fmt, io};

use tracing_core::{
    callsite::{self, Callsite},
    collect, dispatch,
    field::{self, Field, Visit},
    identify_callsite,
    metadata::{Kind, Level},
    Event, Metadata,
};

#[cfg(feature = "log-tracer")]
#[cfg_attr(docsrs, doc(cfg(feature = "log-tracer")))]
pub mod log_tracer;

#[cfg(feature = "log-tracer")]
#[cfg_attr(docsrs, doc(cfg(feature = "log-tracer")))]
#[doc(inline)]
pub use self::log_tracer::LogTracer;

#[cfg(feature = "env_logger")]
#[cfg_attr(docsrs, doc(cfg(feature = "env_logger")))]
pub mod env_logger;

pub use log;

// ~~ @CAD97: BEGIN SMUGGLING HAX ~~
macro_rules! magic_event_name {
    () => {
        // We use two 3 byte noncharacters to specify the magic string over 8 bytes
        // This gives us around ~10 bits of uniqueness space and uses explicitly
        // for internal use codepoints. http://www.unicode.org/faq/private_use.html
        "[\u{FDFE}\u{FDDD}]"
    };
}

/// The magic event name that specifies your event's metadata should be runtime
/// polyfilled with non-`'static` data from the event's visited fields.
pub const MAGIC_EVENT_NAME: &str = magic_event_name!();
/// The magic runtime event metadata field for the name.
pub const MAGIC_EVENT_FIELD_NAME: &str = concat!(magic_event_name!(), " name");
/// The magic runtime event metadata field for the target.
pub const MAGIC_EVENT_FIELD_TARGET: &str = concat!(magic_event_name!(), " target");
/// The magic runtime event metadata field for the level.
pub const MAGIC_EVENT_FIELD_LEVEL: &str = concat!(magic_event_name!(), " level");
/// The magic runtime event metadata field for the file.
pub const MAGIC_EVENT_FIELD_FILE: &str = concat!(magic_event_name!(), " file");
/// The magic runtime event metadata field for the line.
pub const MAGIC_EVENT_FIELD_LINE: &str = concat!(magic_event_name!(), " line");
/// The magic runtime event metadata field for the module path.
pub const MAGIC_EVENT_FIELD_MODULE_PATH: &str = concat!(magic_event_name!(), " module_path");

#[derive(Default)]
struct MagicFields {
    name: Option<field::Field>,
    target: Option<field::Field>,
    level: Option<field::Field>,
    file: Option<field::Field>,
    line: Option<field::Field>,
    module_path: Option<field::Field>,
}

impl MagicFields {
    fn new(metadata: &Metadata<'_>) -> Self {
        let fieldset = metadata.fields();
        let mut fields = fieldset.iter().peekable();
        let mut magic = Self::default();

        let _: Option<()> = (|| {
            if fields.peek()?.name() == MAGIC_EVENT_FIELD_NAME {
                magic.name = fields.next();
            }
            if fields.peek()?.name() == MAGIC_EVENT_FIELD_TARGET {
                magic.target = fields.next();
            }
            if fields.peek()?.name() == MAGIC_EVENT_FIELD_LEVEL {
                magic.level = fields.next();
            }
            if fields.peek()?.name() == MAGIC_EVENT_FIELD_FILE {
                magic.file = fields.next();
            }
            if fields.peek()?.name() == MAGIC_EVENT_FIELD_LINE {
                magic.line = fields.next();
            }
            if fields.peek()?.name() == MAGIC_EVENT_FIELD_MODULE_PATH {
                magic.module_path = fields.next();
            }

            Some(())
        })();

        magic
    }

    fn count(&self) -> usize {
        self.name.is_some() as usize
            + self.target.is_some() as usize
            + self.level.is_some() as usize
            + self.file.is_some() as usize
            + self.line.is_some() as usize
            + self.module_path.is_some() as usize
    }
}

/// A runtime determined name for dynamic event metadata.
/// The actual name itself must be `'static`.
///
/// In order for this to function, this *must* be captured
/// by-Value as a `dyn Error +'static` (in order to downcast).
#[derive(Debug)]
pub struct RuntimeMetadataName(pub &'static str);

// sneaky way to be able to downcast 🤫
impl std::error::Error for RuntimeMetadataName {}
impl fmt::Display for RuntimeMetadataName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
// ~~ @CAD97: END SMUGGLING HAX ~~

/// Format a log record as a trace event in the current span.
pub fn format_trace(record: &log::Record<'_>) -> io::Result<()> {
    dispatch_record(record);
    Ok(())
}

// XXX(eliza): this is factored out so that we don't have to deal with the pub
// function `format_trace`'s `Result` return type...maybe we should get rid of
// that in 0.2...
pub(crate) fn dispatch_record(record: &log::Record<'_>) {
    dispatch::get_default(|dispatch| {
        let filter_meta = record.as_trace();
        if !dispatch.enabled(&filter_meta) {
            return;
        }

        let (_, keys, meta) = loglevel_to_cs(record.level());

        let log_module = record.module_path();
        let log_file = record.file();
        let log_line = record.line();

        let module = log_module.as_ref().map(|s| s as &dyn field::Value);
        let file = log_file.as_ref().map(|s| s as &dyn field::Value);
        let line = log_line.as_ref().map(|s| s as &dyn field::Value);

        let name: &(dyn std::error::Error) = &RuntimeMetadataName("log event");
        dispatch.event(&Event::new(
            meta,
            &meta.fields().value_set(&[
                (&keys.name, Some(&name as &(dyn field::Value))),
                (&keys.message, Some(record.args())),
                (&keys.target, Some(&record.target())),
                (&keys.module, module),
                (&keys.file, file),
                (&keys.line, line),
            ]),
        ));
    });
}

/// Trait implemented for `tracing` types that can be converted to a `log`
/// equivalent.
pub trait AsLog: crate::sealed::Sealed {
    /// The `log` type that this type can be converted into.
    type Log;
    /// Returns the `log` equivalent of `self`.
    fn as_log(&self) -> Self::Log;
}

/// Trait implemented for `log` types that can be converted to a `tracing`
/// equivalent.
pub trait AsTrace: crate::sealed::Sealed {
    /// The `tracing` type that this type can be converted into.
    type Trace;
    /// Returns the `tracing` equivalent of `self`.
    fn as_trace(&self) -> Self::Trace;
}

impl<'a> crate::sealed::Sealed for Metadata<'a> {}

impl<'a> AsLog for Metadata<'a> {
    type Log = log::Metadata<'a>;
    fn as_log(&self) -> Self::Log {
        log::Metadata::builder()
            .level(self.level().as_log())
            .target(self.target())
            .build()
    }
}
impl<'a> crate::sealed::Sealed for log::Metadata<'a> {}

impl<'a> AsTrace for log::Metadata<'a> {
    type Trace = Metadata<'a>;
    fn as_trace(&self) -> Self::Trace {
        let cs_id = identify_callsite!(loglevel_to_cs(self.level()).0);
        Metadata::new(
            "log record",
            self.target(),
            self.level().as_trace(),
            None,
            None,
            None,
            field::FieldSet::new(LOG_FIELD_NAMES, cs_id),
            Kind::EVENT,
        )
    }
}

struct LogFields {
    name: field::Field,
    target: field::Field,
    file: field::Field,
    line: field::Field,
    module: field::Field,
    message: field::Field,
}

static LOG_FIELD_NAMES: &[&str] = &[
    MAGIC_EVENT_FIELD_NAME,
    MAGIC_EVENT_FIELD_TARGET,
    MAGIC_EVENT_FIELD_FILE,
    MAGIC_EVENT_FIELD_LINE,
    MAGIC_EVENT_FIELD_MODULE_PATH,
    "message",
];

impl LogFields {
    fn new(cs: &'static dyn Callsite) -> Self {
        let fieldset = cs.metadata().fields();
        let name = fieldset.field(MAGIC_EVENT_FIELD_NAME).unwrap();
        let target = fieldset.field(MAGIC_EVENT_FIELD_TARGET).unwrap();
        let file = fieldset.field(MAGIC_EVENT_FIELD_FILE).unwrap();
        let line = fieldset.field(MAGIC_EVENT_FIELD_LINE).unwrap();
        let module = fieldset.field(MAGIC_EVENT_FIELD_MODULE_PATH).unwrap();
        let message = fieldset.field("message").unwrap();
        LogFields {
            name,
            message,
            target,
            module,
            file,
            line,
        }
    }
}

macro_rules! log_cs {
    ($level:expr, $cs:ident, $meta:ident, $ty:ident) => {
        struct $ty;
        static $cs: $ty = $ty;
        static $meta: Metadata<'static> = Metadata::new(
            magic_event_name!(),
            "log",
            $level,
            None,
            None,
            None,
            field::FieldSet::new(LOG_FIELD_NAMES, identify_callsite!(&$cs)),
            Kind::EVENT,
        );

        impl callsite::Callsite for $ty {
            fn set_interest(&self, _: collect::Interest) {}
            fn metadata(&self) -> &'static Metadata<'static> {
                &$meta
            }
        }
    };
}

log_cs!(
    tracing_core::Level::TRACE,
    TRACE_CS,
    TRACE_META,
    TraceCallsite
);
log_cs!(
    tracing_core::Level::DEBUG,
    DEBUG_CS,
    DEBUG_META,
    DebugCallsite
);
log_cs!(tracing_core::Level::INFO, INFO_CS, INFO_META, InfoCallsite);
log_cs!(tracing_core::Level::WARN, WARN_CS, WARN_META, WarnCallsite);
log_cs!(
    tracing_core::Level::ERROR,
    ERROR_CS,
    ERROR_META,
    ErrorCallsite
);

lazy_static! {
    static ref TRACE_FIELDS: LogFields = LogFields::new(&TRACE_CS);
    static ref DEBUG_FIELDS: LogFields = LogFields::new(&DEBUG_CS);
    static ref INFO_FIELDS: LogFields = LogFields::new(&INFO_CS);
    static ref WARN_FIELDS: LogFields = LogFields::new(&WARN_CS);
    static ref ERROR_FIELDS: LogFields = LogFields::new(&ERROR_CS);
}

fn level_to_cs(level: Level) -> (&'static dyn Callsite, &'static LogFields) {
    match level {
        Level::TRACE => (&TRACE_CS, &*TRACE_FIELDS),
        Level::DEBUG => (&DEBUG_CS, &*DEBUG_FIELDS),
        Level::INFO => (&INFO_CS, &*INFO_FIELDS),
        Level::WARN => (&WARN_CS, &*WARN_FIELDS),
        Level::ERROR => (&ERROR_CS, &*ERROR_FIELDS),
    }
}

fn loglevel_to_cs(
    level: log::Level,
) -> (
    &'static dyn Callsite,
    &'static LogFields,
    &'static Metadata<'static>,
) {
    match level {
        log::Level::Trace => (&TRACE_CS, &*TRACE_FIELDS, &TRACE_META),
        log::Level::Debug => (&DEBUG_CS, &*DEBUG_FIELDS, &DEBUG_META),
        log::Level::Info => (&INFO_CS, &*INFO_FIELDS, &INFO_META),
        log::Level::Warn => (&WARN_CS, &*WARN_FIELDS, &WARN_META),
        log::Level::Error => (&ERROR_CS, &*ERROR_FIELDS, &ERROR_META),
    }
}

impl<'a> crate::sealed::Sealed for log::Record<'a> {}

impl<'a> AsTrace for log::Record<'a> {
    type Trace = Metadata<'a>;
    fn as_trace(&self) -> Self::Trace {
        let cs_id = identify_callsite!(loglevel_to_cs(self.level()).0);
        Metadata::new(
            "log record",
            self.target(),
            self.level().as_trace(),
            self.file(),
            self.line(),
            self.module_path(),
            field::FieldSet::new(LOG_FIELD_NAMES, cs_id),
            Kind::EVENT,
        )
    }
}

impl crate::sealed::Sealed for tracing_core::Level {}

impl AsLog for tracing_core::Level {
    type Log = log::Level;
    fn as_log(&self) -> log::Level {
        match *self {
            tracing_core::Level::ERROR => log::Level::Error,
            tracing_core::Level::WARN => log::Level::Warn,
            tracing_core::Level::INFO => log::Level::Info,
            tracing_core::Level::DEBUG => log::Level::Debug,
            tracing_core::Level::TRACE => log::Level::Trace,
        }
    }
}

impl crate::sealed::Sealed for log::Level {}

impl AsTrace for log::Level {
    type Trace = tracing_core::Level;
    #[inline]
    fn as_trace(&self) -> tracing_core::Level {
        match self {
            log::Level::Error => tracing_core::Level::ERROR,
            log::Level::Warn => tracing_core::Level::WARN,
            log::Level::Info => tracing_core::Level::INFO,
            log::Level::Debug => tracing_core::Level::DEBUG,
            log::Level::Trace => tracing_core::Level::TRACE,
        }
    }
}

impl crate::sealed::Sealed for log::LevelFilter {}

impl AsTrace for log::LevelFilter {
    type Trace = tracing_core::LevelFilter;
    #[inline]
    fn as_trace(&self) -> tracing_core::LevelFilter {
        match self {
            log::LevelFilter::Off => tracing_core::LevelFilter::OFF,
            log::LevelFilter::Error => tracing_core::LevelFilter::ERROR,
            log::LevelFilter::Warn => tracing_core::LevelFilter::WARN,
            log::LevelFilter::Info => tracing_core::LevelFilter::INFO,
            log::LevelFilter::Debug => tracing_core::LevelFilter::DEBUG,
            log::LevelFilter::Trace => tracing_core::LevelFilter::TRACE,
        }
    }
}

impl crate::sealed::Sealed for tracing_core::LevelFilter {}

impl AsLog for tracing_core::LevelFilter {
    type Log = log::LevelFilter;
    #[inline]
    fn as_log(&self) -> Self::Log {
        match *self {
            tracing_core::LevelFilter::OFF => log::LevelFilter::Off,
            tracing_core::LevelFilter::ERROR => log::LevelFilter::Error,
            tracing_core::LevelFilter::WARN => log::LevelFilter::Warn,
            tracing_core::LevelFilter::INFO => log::LevelFilter::Info,
            tracing_core::LevelFilter::DEBUG => log::LevelFilter::Debug,
            tracing_core::LevelFilter::TRACE => log::LevelFilter::Trace,
        }
    }
}

/// Extends log `Event`s to provide complete `Metadata`.
///
/// `Event` requires its `Metadata` to be `'static`, but sometimes you need to
/// provide metadatata which is dynamic at runtime, such as [`log::Record`]'s,
/// which is provided with a generic lifetime.
///
/// In order to facilitate such, we allow metadata to set its name to the magic
/// [`MAGIC_EVENT_NAME`]. Then, the use of this trait to normalize the event
/// can recognize specially named fields and use them to override metadatata
/// for the returned short-lived metadata.
///
/// When procesing an event, you can use [`normalized_metadata`] to get the
/// complete view of the metadata after normalization, without any magic fields
/// used to encode the runtime metadata.
///
/// # How to Provide Dynamic Metadata
///
/// The opt-in is to set your metadata's name to [`MAGIC_EVENT_NAME`]. Only
/// events with this magic name are processed. (The name is constructed of valid
/// UTF-8 [noncharacters], so it should never show up in interchange UTF-8.)
///
/// Then, the prefix of the metadata's field set is checked for the following
/// magic fields, *in order*:
///
/// - [`MAGIC_EVENT_FIELD_NAME`]: If present, this indicates a new name to use
///   for [`Metadata::name`]. Must be a [`RuntimeMetadataName`] captured as a
///   `dyn 'static + Error` value (so that it can be downcast).
///
/// - [`MAGIC_EVENT_FIELD_TARGET`]: If present, this indicates a new target to
///   use for [`Metadata::target`]. Must be captured as a `&str` value.
///     <div class="example-wrap" style="display:inline-block">
///     <pre class="ignore" style="white-space:normal;font:inherit;">
///
///     **Note**: Although it is possible to override the target, it is
///     generally advisable to make the initial target as accurate as possible,
///     as static filtering is done with the static metadata's target through
///     [`register_callsite`].
///     </pre></div>
///
/// - [`MAGIC_EVENT_FIELD_LEVEL`]: If present, this indicates a new level to use
///   for [`Metadata::level`]. Must be captured as a `&str` value, which is
///   parsed using [`str::parse`].
///     <div class="example-wrap" style="display:inline-block">
///     <pre class="ignore" style="white-space:normal;font:inherit;">
///
///     **Note**: Although it is possible to override the level, it is generally
///     advisable not to, as static filtering is done with the static metadata's
///     level through [`register_callsite`].
///     </pre></div>
///
/// - [`MAGIC_EVENT_FIELD_FILE`]: If present, this indicates a new file to use
///   for [`Metadata::file`]. Must be captured as a `&str` value.
/// - [`MAGIC_EVENT_FIELD_LINE`]: If present, this indicates a new line to use
///   for [`Metadata::line`]. Must be captured as a `u64` value.
/// - [`MAGIC_EVENT_FIELD_MODULE_PATH`]: If present, this indicates a new
///   module path to use for [`Metadata::module_path`]. Must be captured as a
///   `&str` value.
///
/// Any remaining fields after the magic fields are processed (such as the
/// common `message` field) are passed on through to the normalized metadata.
/// Note, however, that magic fields ***must not*** be mixed with passthrough
/// fields; only a contiguous prefix of magic fields are processed.
///
/// [noncharacters]: http://www.unicode.org/faq/private_use.html#nonchar1
/// [`normalized_metadata`]: NormalizeEvent#normalized_metadata
/// [`register_callsite`]: tracing_core::Collect::register_callsite
pub trait NormalizeEvent<'a>: crate::sealed::Sealed {
    /// If this `Event` comes from a `log`, this method provides a new
    /// normalized `Metadata` which has all available attributes
    /// from the original log, including `file`, `line`, `module_path`
    /// and `target`.
    /// Returns `None` is the `Event` is not issued from a `log`.
    fn normalized_metadata(&'a self) -> Option<Metadata<'a>>;

    /// Returns whether this `Event` represents a log (from the `log` crate)
    #[deprecated]
    fn is_log(&self) -> bool;
}

impl<'a> crate::sealed::Sealed for Event<'a> {}

impl<'a> NormalizeEvent<'a> for Event<'a> {
    // ~~ @CAD97: BEGIN SMUGGLING HAX ~~
    fn normalized_metadata(&'a self) -> Option<Metadata<'a>> {
        let original = self.metadata();
        if original.name() != magic_event_name!() {
            return None;
        }

        struct MagicVisitor<'a> {
            name: &'static str,
            target: &'a str,
            level: Level,
            file: Option<&'a str>,
            line: Option<u32>,
            module_path: Option<&'a str>,
            fields: MagicFields,
        }

        let mut visitor = MagicVisitor {
            name: original.name(),
            target: original.target(),
            level: *original.level(),
            file: original.file(),
            line: original.line(),
            module_path: original.module_path(),
            fields: MagicFields::new(original),
        };

        self.record(&mut visitor);
        return Some(Metadata::new(
            visitor.name,
            visitor.target,
            visitor.level,
            visitor.file,
            visitor.line,
            visitor.module_path,
            original.fields().slice(visitor.fields.count()..),
            Kind::EVENT,
        ));

        impl Visit for MagicVisitor<'_> {
            fn record_debug(&mut self, _field: &Field, _value: &dyn fmt::Debug) {}

            fn record_u64(&mut self, field: &Field, value: u64) {
                if Some(field) == self.fields.line.as_ref() {
                    self.line = Some(value.try_into().unwrap_or(u32::MAX));
                }
            }

            fn record_str(&mut self, field: &Field, value: &str) {
                unsafe {
                    // The `Visit` API erases the string slice's lifetime. However, we
                    // know it is part of the `Event` struct with a lifetime of `'a`. If
                    // (and only if!) this `MagicVisitor` was constructed with the same
                    // lifetime parameter `'a` as the event in question, it's safe to
                    // cast these string slices to the `'a` lifetime.
                    if Some(field) == self.fields.target.as_ref() {
                        self.target = &*(value as *const _);
                    } else if Some(field) == self.fields.level.as_ref() {
                        self.level = value.parse().unwrap_or(self.level);
                    } else if Some(field) == self.fields.file.as_ref() {
                        self.file = Some(&*(value as *const _));
                    } else if Some(field) == self.fields.module_path.as_ref() {
                        self.module_path = Some(&*(value as *const _));
                    }
                }
            }

            fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
                if Some(field) == self.fields.name.as_ref() {
                    if let Some(value) = value.downcast_ref::<RuntimeMetadataName>() {
                        self.name = value.0;
                    }
                }
            }
        }
    }
    // ~~ @CAD97: END SMUGGLING HAX ~~

    fn is_log(&self) -> bool {
        self.metadata().callsite() == identify_callsite!(level_to_cs(*self.metadata().level()).0)
    }
}

mod sealed {
    pub trait Sealed {}
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_callsite(level: log::Level) {
        let record = log::Record::builder()
            .args(format_args!("Error!"))
            .level(level)
            .target("myApp")
            .file(Some("server.rs"))
            .line(Some(144))
            .module_path(Some("server"))
            .build();

        let meta = record.as_trace();
        let (cs, _keys, _) = loglevel_to_cs(record.level());
        let cs_meta = cs.metadata();
        assert_eq!(
            meta.callsite(),
            cs_meta.callsite(),
            "actual: {:#?}\nexpected: {:#?}",
            meta,
            cs_meta
        );
        assert_eq!(meta.level(), &level.as_trace());
    }

    #[test]
    fn error_callsite_is_correct() {
        test_callsite(log::Level::Error);
    }

    #[test]
    fn warn_callsite_is_correct() {
        test_callsite(log::Level::Warn);
    }

    #[test]
    fn info_callsite_is_correct() {
        test_callsite(log::Level::Info);
    }

    #[test]
    fn debug_callsite_is_correct() {
        test_callsite(log::Level::Debug);
    }

    #[test]
    fn trace_callsite_is_correct() {
        test_callsite(log::Level::Trace);
    }
}
