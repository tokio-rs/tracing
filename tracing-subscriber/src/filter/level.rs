use std::{cmp::Ordering, fmt, str::FromStr};
use tracing_core::Level;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LevelFilter(Inner);

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Inner {
    Off,
    Level(Level),
}

#[derive(Clone, Debug)]
pub struct ParseError(());

// === impl LevelFilter ===

impl LevelFilter {
    /// The "off" level.
    ///
    /// Designates that trace instrumentation should be completely disabled.
    pub const OFF: LevelFilter = LevelFilter(Inner::Off);
    /// The "error" level.
    ///
    /// Designates very serious errors.
    pub const ERROR: LevelFilter = LevelFilter(Inner::Level(Level::ERROR));
    /// The "warn" level.
    ///
    /// Designates hazardous situations.
    pub const WARN: LevelFilter = LevelFilter(Inner::Level(Level::WARN));
    /// The "info" level.
    ///
    /// Designates useful information.
    pub const INFO: LevelFilter = LevelFilter(Inner::Level(Level::INFO));
    /// The "debug" level.
    ///
    /// Designates lower priority information.
    pub const DEBUG: LevelFilter = LevelFilter(Inner::Level(Level::DEBUG));
    /// The "trace" level.
    ///
    /// Designates very low priority, often extremely verbose, information.
    pub const TRACE: LevelFilter = LevelFilter(Inner::Level(Level::TRACE));
}

impl PartialEq<Level> for LevelFilter {
    fn eq(&self, other: &Level) -> bool {
        match self.0 {
            Inner::Off => false,
            Inner::Level(ref level) => level == other,
        }
    }
}

impl PartialOrd<Level> for LevelFilter {
    fn partial_cmp(&self, other: &Level) -> Option<Ordering> {
        match self.0 {
            Inner::Off => Some(Ordering::Less),
            Inner::Level(ref level) => level.partial_cmp(other),
        }
    }
}

impl fmt::Display for LevelFilter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Inner::Off => f.pad("OFF"),
            Inner::Level(Level::ERROR) => f.pad("ERROR"),
            Inner::Level(Level::WARN) => f.pad("WARM"),
            Inner::Level(Level::INFO) => f.pad("INFO"),
            Inner::Level(Level::DEBUG) => f.pad("DEBUG"),
            Inner::Level(Level::TRACE) => f.pad("TRACE"),
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

impl Into<Option<Level>> for LevelFilter {
    fn into(self) -> Option<Level> {
        match self.0 {
            Inner::Off => None,
            Inner::Level(l) => Some(l),
        }
    }
}

impl From<Option<Level>> for LevelFilter {
    fn from(level: Option<Level>) -> Self {
        match level {
            Some(level) => LevelFilter(Inner::Level(level)),
            None => LevelFilter(Inner::Off),
        }
    }
}

impl From<Level> for LevelFilter {
    fn from(level: Level) -> Self {
        LevelFilter(Inner::Level(level))
    }
}

// === impl ParseError ===

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid level`")
    }
}

impl std::error::Error for ParseError {}
