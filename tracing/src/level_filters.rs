//! Trace verbosity level filtering.
//!
//! # Compile time filters
//!
//! Trace verbosity levels can be statically disabled at compile time via Cargo
//! features, similar to the [`log` crate]. Trace instrumentation at disabled
//! levels will be skipped and will not even be present in the resulting binary
//! unless the verbosity level is specified dynamically. This level is
//! configured separately for release and debug builds. The features are:
//!
//! * `max_level_off`
//! * `max_level_error`
//! * `max_level_warn`
//! * `max_level_info`
//! * `max_level_debug`
//! * `max_level_trace`
//! * `release_max_level_off`
//! * `release_max_level_error`
//! * `release_max_level_warn`
//! * `release_max_level_info`
//! * `release_max_level_debug`
//! * `release_max_level_trace`
//!
//! These features control the value of the `STATIC_MAX_LEVEL` constant. The
//! instrumentation macros macros check this value before recording an event or
//! constructing a span. By default, no levels are disabled.
//!
//! For example, a crate can disable trace level instrumentation in debug builds
//! and trace, debug, and info level instrumentation in release builds with the
//! following configuration:
//!
//! ```toml
//! [dependencies]
//! tracing = { version = "0.1", features = ["max_level_debug", "release_max_level_warn"] }
//! ```
//!
//! *Compiler support: requires rustc 1.39+*
//!
//! [`log` crate]: https://docs.rs/log/0.4.6/log/#compile-time-filters
use crate::stdlib::cmp::Ordering;
use tracing_core::Level;

/// A filter comparable to trace verbosity `Level`.
///
/// If a `Level` is considered less than a `LevelFilter`, it should be
/// considered disabled; if greater than or equal to the `LevelFilter`, that
/// level is enabled.
///
/// Note that this is essentially identical to the `Level` type, but with the
/// addition of an `OFF` level that completely disables all trace
/// instrumentation.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LevelFilter(Option<Level>);

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
    fn partial_cmp(&self, other: &LevelFilter) -> Option<Ordering> {
        match other.0 {
            None => Some(Ordering::Less),
            Some(ref level) => self.partial_cmp(level),
        }
    }
}

/// The statically configured maximum trace level.
///
/// See the [module-level documentation] for information on how to configure
/// this.
///
/// This value is checked by the `event!` and `span!` macros. Code that
/// manually constructs events or spans via the `Event::record` function or
/// `Span` constructors should compare the level against this value to
/// determine if those spans or events are enabled.
///
/// [module-level documentation]: ../index.html#compile-time-filters
pub const STATIC_MAX_LEVEL: LevelFilter = MAX_LEVEL;

cfg_if! {
    if #[cfg(all(not(debug_assertions), feature = "release_max_level_off"))] {
        const MAX_LEVEL: LevelFilter = LevelFilter::OFF;
    } else if #[cfg(all(not(debug_assertions), feature = "release_max_level_error"))] {
        const MAX_LEVEL: LevelFilter = LevelFilter::ERROR;
    } else if #[cfg(all(not(debug_assertions), feature = "release_max_level_warn"))] {
        const MAX_LEVEL: LevelFilter = LevelFilter::WARN;
    } else if #[cfg(all(not(debug_assertions), feature = "release_max_level_info"))] {
        const MAX_LEVEL: LevelFilter = LevelFilter::INFO;
    } else if #[cfg(all(not(debug_assertions), feature = "release_max_level_debug"))] {
        const MAX_LEVEL: LevelFilter = LevelFilter::DEBUG;
    } else if #[cfg(all(not(debug_assertions), feature = "release_max_level_trace"))] {
        const MAX_LEVEL: LevelFilter = LevelFilter::TRACE;
    } else if #[cfg(feature = "max_level_off")] {
        const MAX_LEVEL: LevelFilter = LevelFilter::OFF;
    } else if #[cfg(feature = "max_level_error")] {
        const MAX_LEVEL: LevelFilter = LevelFilter::ERROR;
    } else if #[cfg(feature = "max_level_warn")] {
        const MAX_LEVEL: LevelFilter = LevelFilter::WARN;
    } else if #[cfg(feature = "max_level_info")] {
        const MAX_LEVEL: LevelFilter = LevelFilter::INFO;
    } else if #[cfg(feature = "max_level_debug")] {
        const MAX_LEVEL: LevelFilter = LevelFilter::DEBUG;
    } else {
        const MAX_LEVEL: LevelFilter = LevelFilter::TRACE;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_level_conversion() {
        let mapping = [
            (LevelFilter::OFF, None),
            (LevelFilter::ERROR, Some(Level::ERROR)),
            (LevelFilter::WARN, Some(Level::WARN)),
            (LevelFilter::INFO, Some(Level::INFO)),
            (LevelFilter::DEBUG, Some(Level::DEBUG)),
            (LevelFilter::TRACE, Some(Level::TRACE)),
        ];
        for (filter, level) in mapping.iter() {
            assert_eq!(filter.clone().into_level(), *level);
            if let Some(level) = level {
                assert_eq!(LevelFilter::from_level(level.clone()), *filter);
            }
        }
    }
}
