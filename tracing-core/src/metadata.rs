//! Metadata describing trace data.
use super::{callsite, field};
use std::{fmt, str::FromStr};

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
/// filtering is based on metadata, rather than  on the constructed span.
///
/// **Note**: Although instances of `Metadata` cannot be compared directly, they
/// provide a method [`id`] which returns an opaque [callsite identifier]
/// which uniquely identifies the callsite where the metadata originated.
/// This can be used for determining if two Metadata correspond to
/// the same callsite.
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

impl std::error::Error for ParseLevelError {}

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
    Error = 1,
    /// The "warn" level.
    ///
    /// Designates hazardous situations.
    Warn,
    /// The "info" level.
    ///
    /// Designates useful information.
    Info,
    /// The "debug" level.
    ///
    /// Designates lower priority information.
    Debug,
    /// The "trace" level.
    ///
    /// Designates very low priority, often extremely verbose, information.
    Trace,
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
