//! [`Subscriber`]s that control which spans and events are enabled by the wrapped
//! subscriber.
//!
//! [`Subscriber`]: ../layer/trait.Subscriber.html
#[cfg(feature = "env-filter")]
mod env;
mod level;

pub use self::level::{LevelFilter, ParseError as LevelParseError};

#[cfg(feature = "env-filter")]
#[cfg_attr(docsrs, doc(cfg(feature = "env-filter")))]
pub use self::env::*;
