//! Metadata describing trace data.
use super::{callsite, field};
use crate::stdlib::{
    cmp, fmt,
    str::FromStr,
    sync::atomic::{AtomicUsize, Ordering},
};

/// Metadata describing a [span] or [event].
///
/// All spans and events have the following metadata:
/// - A [name], represented as a static string.
/// - A [target], a string that categorizes part of the system where the span
///   or event occurred. The `tracing` macros default to using the module
///   path where the span or event originated as the target, but it may be
///   overridden.
/// - A [verbosity level].
/// - The names of the [fields] defined by the span or event.
/// - Whether the metadata corresponds to a span or event.
///
/// In addition, the following optional metadata describing the source code
/// location where the span or event originated _may_ be provided:
/// - The [file name]
/// - The [line number]
/// - The [module path]
///
/// Metadata is used by [`Subscriber`]s when filtering spans and events, and it
/// may also be used as part of their data payload.
///
/// When created by the `event!` or `span!` macro, the metadata describing a
/// particular event or span is constructed statically and exists as a single
/// static instance. Thus, the overhead of creating the metadata is
/// _significantly_ lower than that of creating the actual span. Therefore,
/// filtering is based on metadata, rather than on the constructed span.
///
/// <div class="information">
///     <div class="tooltip ignore" style="">ⓘ<span class="tooltiptext">Note</span></div>
/// </div>
/// <div class="example-wrap" style="display:inline-block">
/// <pre class="ignore" style="white-space:normal;font:inherit;">
/// <strong>Note</strong>: Although instances of <code>Metadata</code> cannot
/// be compared directly, they provide a method <a href="struct.Metadata.html#method.id">
/// <code>id</code></a>, returning an opaque <a href="../callsite/struct.Identifier.html">
/// callsite identifier</a>  which uniquely identifies the callsite where the metadata
/// originated. This can be used to determine if two <code>Metadata</code> correspond to
/// the same callsite.
/// </pre></div>
///
/// [span]: ../span/index.html
/// [event]: ../event/index.html
/// [name]: #method.name
/// [target]: #method.target
/// [fields]: #method.fields
/// [verbosity level]: #method.level
/// [file name]: #method.file
/// [line number]: #method.line
/// [module path]: #method.module
/// [`Subscriber`]: ../subscriber/trait.Subscriber.html
/// [`id`]: struct.Metadata.html#method.id
/// [callsite identifier]: ../callsite/struct.Identifier.html
pub struct Metadata<'a> {
    /// The name of the span described by this metadata.
    name: &'static str,

    /// The part of the system that the span that this metadata describes
    /// occurred in.
    target: &'a str,

    /// The level of verbosity of the described span.
    level: Level,

    /// The name of the Rust module where the span occurred, or `None` if this
    /// could not be determined.
    module_path: Option<&'a str>,

    /// The name of the source code file where the span occurred, or `None` if
    /// this could not be determined.
    file: Option<&'a str>,

    /// The line number in the source code file where the span occurred, or
    /// `None` if this could not be determined.
    line: Option<u32>,

    /// The names of the key-value fields attached to the described span or
    /// event.
    fields: field::FieldSet,

    /// The kind of the callsite.
    kind: Kind,
}

/// Indicates whether the callsite is a span or event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Kind(KindInner);

/// Describes the level of verbosity of a span or event.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Level(LevelInner);

/// A filter comparable to a verbosity `Level`.
///
/// If a `Level` is considered less than a `LevelFilter`, it should be
/// considered disabled; if greater than or equal to the `LevelFilter`, that
/// level is enabled.
///
/// Note that this is essentially identical to the `Level` type, but with the
/// addition of an `OFF` level that completely disables all trace
/// instrumentation.
#[repr(transparent)]
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct LevelFilter(Option<Level>);

/// Indicates that a string could not be parsed to a valid level.
#[derive(Clone, Debug)]
pub struct ParseError(());

static MAX_LEVEL: AtomicUsize = AtomicUsize::new(LevelFilter::OFF_USIZE);

// ===== impl Metadata =====

impl<'a> Metadata<'a> {
    /// Construct new metadata for a span or event, with a name, target, level, field
    /// names, and optional source code location.
    pub const fn new(
        name: &'static str,
        target: &'a str,
        level: Level,
        file: Option<&'a str>,
        line: Option<u32>,
        module_path: Option<&'a str>,
        fields: field::FieldSet,
        kind: Kind,
    ) -> Self {
        Metadata {
            name,
            target,
            level,
            module_path,
            file,
            line,
            fields,
            kind,
        }
    }

    /// Returns the names of the fields on the described span or event.
    pub fn fields(&self) -> &field::FieldSet {
        &self.fields
    }

    /// Returns the level of verbosity of the described span or event.
    pub fn level(&self) -> &Level {
        &self.level
    }

    /// Returns the name of the span.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Returns a string describing the part of the system where the span or
    /// event that this metadata describes occurred.
    ///
    /// Typically, this is the module path, but alternate targets may be set
    /// when spans or events are constructed.
    pub fn target(&self) -> &'a str {
        self.target
    }

    /// Returns the path to the Rust module where the span occurred, or
    /// `None` if the module path is unknown.
    pub fn module_path(&self) -> Option<&'a str> {
        self.module_path
    }

    /// Returns the name of the source code file where the span
    /// occurred, or `None` if the file is unknown
    pub fn file(&self) -> Option<&'a str> {
        self.file
    }

    /// Returns the line number in the source code file where the span
    /// occurred, or `None` if the line number is unknown.
    pub fn line(&self) -> Option<u32> {
        self.line
    }

    /// Returns an opaque `Identifier` that uniquely identifies the callsite
    /// this `Metadata` originated from.
    #[inline]
    pub fn callsite(&self) -> callsite::Identifier {
        self.fields.callsite()
    }

    /// Returns true if the callsite kind is `Event`.
    pub fn is_event(&self) -> bool {
        self.kind.is_event()
    }

    /// Return true if the callsite kind is `Span`.
    pub fn is_span(&self) -> bool {
        self.kind.is_span()
    }
}

impl<'a> fmt::Debug for Metadata<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut meta = f.debug_struct("Metadata");
        meta.field("name", &self.name)
            .field("target", &self.target)
            .field("level", &self.level);

        if let Some(path) = self.module_path() {
            meta.field("module_path", &path);
        }

        match (self.file(), self.line()) {
            (Some(file), Some(line)) => {
                meta.field("location", &format_args!("{}:{}", file, line));
            }
            (Some(file), None) => {
                meta.field("file", &format_args!("{}", file));
            }

            // Note: a line num with no file is a kind of weird case that _probably_ never occurs...
            (None, Some(line)) => {
                meta.field("line", &line);
            }
            (None, None) => {}
        };

        meta.field("fields", &format_args!("{}", self.fields))
            .field("callsite", &self.callsite())
            .field("kind", &self.kind)
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum KindInner {
    Event,
    Span,
}

impl Kind {
    /// `Event` callsite
    pub const EVENT: Kind = Kind(KindInner::Event);

    /// `Span` callsite
    pub const SPAN: Kind = Kind(KindInner::Span);

    /// Return true if the callsite kind is `Span`
    pub fn is_span(&self) -> bool {
        match self {
            Kind(KindInner::Span) => true,
            _ => false,
        }
    }

    /// Return true if the callsite kind is `Event`
    pub fn is_event(&self) -> bool {
        match self {
            Kind(KindInner::Event) => true,
            _ => false,
        }
    }
}

// ===== impl Level =====

impl Level {
    /// The "error" level.
    ///
    /// Designates very serious errors.
    pub const ERROR: Level = Level(LevelInner::Error);
    /// The "warn" level.
    ///
    /// Designates hazardous situations.
    pub const WARN: Level = Level(LevelInner::Warn);
    /// The "info" level.
    ///
    /// Designates useful information.
    pub const INFO: Level = Level(LevelInner::Info);
    /// The "debug" level.
    ///
    /// Designates lower priority information.
    pub const DEBUG: Level = Level(LevelInner::Debug);
    /// The "trace" level.
    ///
    /// Designates very low priority, often extremely verbose, information.
    pub const TRACE: Level = Level(LevelInner::Trace);
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Level::TRACE => f.pad("TRACE"),
            Level::DEBUG => f.pad("DEBUG"),
            Level::INFO => f.pad("INFO"),
            Level::WARN => f.pad("WARN"),
            Level::ERROR => f.pad("ERROR"),
        }
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl crate::stdlib::error::Error for ParseLevelError {}

impl FromStr for Level {
    type Err = ParseLevelError;
    fn from_str(s: &str) -> Result<Self, ParseLevelError> {
        s.parse::<usize>()
            .map_err(|_| ParseLevelError { _p: () })
            .and_then(|num| match num {
                1 => Ok(Level::ERROR),
                2 => Ok(Level::WARN),
                3 => Ok(Level::INFO),
                4 => Ok(Level::DEBUG),
                5 => Ok(Level::TRACE),
                _ => Err(ParseLevelError { _p: () }),
            })
            .or_else(|_| match s {
                s if s.eq_ignore_ascii_case("error") => Ok(Level::ERROR),
                s if s.eq_ignore_ascii_case("warn") => Ok(Level::WARN),
                s if s.eq_ignore_ascii_case("info") => Ok(Level::INFO),
                s if s.eq_ignore_ascii_case("debug") => Ok(Level::DEBUG),
                s if s.eq_ignore_ascii_case("trace") => Ok(Level::TRACE),
                _ => Err(ParseLevelError { _p: () }),
            })
    }
}

#[repr(usize)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
enum LevelInner {
    /// The "error" level.
    ///
    /// Designates very serious errors.
    Error = 0,
    /// The "warn" level.
    ///
    /// Designates hazardous situations.
    Warn = 1,
    /// The "info" level.
    ///
    /// Designates useful information.
    Info = 2,
    /// The "debug" level.
    ///
    /// Designates lower priority information.
    Debug = 3,
    /// The "trace" level.
    ///
    /// Designates very low priority, often extremely verbose, information.
    Trace = 4,
}

// === impl LevelFilter ===

impl From<Level> for LevelFilter {
    #[inline]
    fn from(level: Level) -> Self {
        Self::from_level(level)
    }
}

impl LevelFilter {
    /// The "off" level.
    ///
    /// Designates that trace instrumentation should be completely disabled.
    pub const OFF: LevelFilter = LevelFilter(None);
    /// The "error" level.
    ///
    /// Designates very serious errors.
    pub const ERROR: LevelFilter = LevelFilter::from_level(Level::ERROR);
    /// The "warn" level.
    ///
    /// Designates hazardous situations.
    pub const WARN: LevelFilter = LevelFilter::from_level(Level::WARN);
    /// The "info" level.
    ///
    /// Designates useful information.
    pub const INFO: LevelFilter = LevelFilter::from_level(Level::INFO);
    /// The "debug" level.
    ///
    /// Designates lower priority information.
    pub const DEBUG: LevelFilter = LevelFilter::from_level(Level::DEBUG);
    /// The "trace" level.
    ///
    /// Designates very low priority, often extremely verbose, information.
    pub const TRACE: LevelFilter = LevelFilter(Some(Level::TRACE));

    /// Returns a `LevelFilter` that enables spans and events with verbosity up
    /// to and including `level`.
    pub const fn from_level(level: Level) -> Self {
        Self(Some(level))
    }

    /// Returns the most verbose [`Level`] that this filter accepts, or `None`
    /// if it is [`OFF`].
    ///
    /// [`Level`]: ../struct.Level.html
    /// [`OFF`]: #associatedconstant.OFF
    pub const fn into_level(self) -> Option<Level> {
        self.0
    }

    // These consts are necessary because `as` casts are not allowed as
    // match patterns.
    const ERROR_USIZE: usize = LevelInner::Error as usize;
    const WARN_USIZE: usize = LevelInner::Warn as usize;
    const INFO_USIZE: usize = LevelInner::Info as usize;
    const DEBUG_USIZE: usize = LevelInner::Debug as usize;
    const TRACE_USIZE: usize = LevelInner::Trace as usize;
    // Using the value of the last variant + 1 ensures that we match the value
    // for `Option::None` as selected by the niche optimization for
    // `LevelFilter`. If this is the case, converting a `usize` value into a
    // `LevelFilter` (in `LevelFilter::max`) will be an identity conversion,
    // rather than generating a lookup table.
    const OFF_USIZE: usize = LevelInner::Trace as usize + 1;

    /// Returns a `LevelFilter` that matches the most verbose [`Level`] that any
    /// currently active [`Subscriber`] will enable.
    #[inline(always)]
    pub fn max() -> Self {
        match MAX_LEVEL.load(Ordering::Relaxed) {
            Self::ERROR_USIZE => Self::ERROR,
            Self::WARN_USIZE => Self::WARN,
            Self::INFO_USIZE => Self::INFO,
            Self::DEBUG_USIZE => Self::DEBUG,
            Self::TRACE_USIZE => Self::TRACE,
            Self::OFF_USIZE => Self::OFF,
            _ => unsafe {
                // Using `unreachable_unchecked` here (rather than
                // `unreachable!()`) is necessary to ensure that rustc generates
                // an identity conversion from integer -> discriminant, rather
                // than generating a lookup table. We want to ensure this
                // function is a single `mov` instruction (on x86) if at all
                // possible, because it is called *every* time a span/event
                // callsite is hit; and it is (potentially) the only code in the
                // hottest path for skipping a majority of callsites when level
                // filtering is in use.
                //
                // safety: This branch is only truly unreachable if we guarantee
                // that no values other than the possible enum discriminants
                // will *ever* be present. The `AtomicUsize` is initialized to
                // the `OFF` value. It is only set by the `set_max` function,
                // which takes a `LevelFilter` as a parameter. This restricts
                // the inputs to `set_max` to the set of valid discriminants.
                // Therefore, **as long as `MAX_VALUE` is only ever set by
                // `set_max`**, this is safe.
                core::hint::unreachable_unchecked()
            },
        }
    }

    pub(crate) fn set_max(LevelFilter(level): LevelFilter) {
        let val = match level {
            Some(Level(level)) => level as usize,
            None => Self::OFF_USIZE,
        };

        // using an AcqRel swap ensures an ordered relationship of writes to the
        // max level.
        MAX_LEVEL.swap(val, Ordering::AcqRel);
    }
}

impl PartialEq<LevelFilter> for Level {
    fn eq(&self, other: &LevelFilter) -> bool {
        match other.0 {
            None => false,
            Some(ref level) => self.eq(level),
        }
    }
}

impl PartialOrd<LevelFilter> for Level {
    fn partial_cmp(&self, other: &LevelFilter) -> Option<cmp::Ordering> {
        match other.0 {
            None => Some(cmp::Ordering::Less),
            Some(ref level) => self.partial_cmp(level),
        }
    }
}

impl PartialOrd<Level> for LevelFilter {
    fn partial_cmp(&self, other: &Level) -> Option<cmp::Ordering> {
        match self.0 {
            None => Some(cmp::Ordering::Greater),
            Some(ref level) => level.partial_cmp(other),
        }
    }
}

impl PartialEq<Level> for LevelFilter {
    fn eq(&self, other: &Level) -> bool {
        match self.0 {
            None => false,
            Some(ref level) => level.eq(other),
        }
    }
}

impl fmt::Display for LevelFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            LevelFilter::OFF => f.pad("off"),
            LevelFilter::ERROR => f.pad("error"),
            LevelFilter::WARN => f.pad("warn"),
            LevelFilter::INFO => f.pad("info"),
            LevelFilter::DEBUG => f.pad("debug"),
            LevelFilter::TRACE => f.pad("trace"),
        }
    }
}

impl fmt::Debug for LevelFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            LevelFilter::OFF => f.pad("LevelFilter::OFF"),
            LevelFilter::ERROR => f.pad("LevelFilter::ERROR"),
            LevelFilter::WARN => f.pad("LevelFilter::WARN"),
            LevelFilter::INFO => f.pad("LevelFilter::INFO"),
            LevelFilter::DEBUG => f.pad("LevelFilter::DEBUG"),
            LevelFilter::TRACE => f.pad("LevelFilter::TRACE"),
        }
    }
}

impl FromStr for LevelFilter {
    type Err = ParseError;
    fn from_str(from: &str) -> Result<Self, Self::Err> {
        from.parse::<usize>()
            .ok()
            .and_then(|num| match num {
                0 => Some(LevelFilter::OFF),
                1 => Some(LevelFilter::ERROR),
                2 => Some(LevelFilter::WARN),
                3 => Some(LevelFilter::INFO),
                4 => Some(LevelFilter::DEBUG),
                5 => Some(LevelFilter::TRACE),
                _ => None,
            })
            .or_else(|| match from {
                "" => Some(LevelFilter::ERROR),
                s if s.eq_ignore_ascii_case("error") => Some(LevelFilter::ERROR),
                s if s.eq_ignore_ascii_case("warn") => Some(LevelFilter::WARN),
                s if s.eq_ignore_ascii_case("info") => Some(LevelFilter::INFO),
                s if s.eq_ignore_ascii_case("debug") => Some(LevelFilter::DEBUG),
                s if s.eq_ignore_ascii_case("trace") => Some(LevelFilter::TRACE),
                s if s.eq_ignore_ascii_case("off") => Some(LevelFilter::OFF),
                _ => None,
            })
            .ok_or_else(|| ParseError(()))
    }
}

/// Returned if parsing a `Level` fails.
#[derive(Debug)]
pub struct ParseLevelError {
    _p: (),
}

impl fmt::Display for ParseLevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(
            "error parsing level: expected one of \"error\", \"warn\", \
             \"info\", \"debug\", \"trace\", or a number 1-5",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_from_str() {
        assert_eq!("error".parse::<Level>().unwrap(), Level::ERROR);
        assert_eq!("4".parse::<Level>().unwrap(), Level::DEBUG);
        assert!("0".parse::<Level>().is_err())
    }
}
