//! [`Layer`]s that control which spans and events are enabled by the wrapped
//! subscriber.
//!
//! [`Layer`]: ../trait.Layer.html
#[cfg(feature = "filter")]
mod env;
mod level;

pub use self::level::{LevelFilter, ParseError as LevelParseError};

#[cfg(feature = "filter")]
pub use self::env::*;

/// A `Layer` which filters spans and events based on a set of filter
/// directives.
///
/// **Note**: renamed to `EnvFilter` in 0.1.2; use that instead.
#[cfg(feature = "filter")]
#[deprecated(since = "0.1.2", note = "renamed to `EnvFilter`")]
pub type Filter = EnvFilter;
