//! [`Layer`]s that control which spans and events are enabled by the wrapped
//! subscriber.
//!
//! [`Layer`]: ../trait.Layer.html
#[cfg(feature = "env-filter")]
mod env;
mod level;

use std::sync::Arc;

pub use self::level::{LevelFilter, ParseError as LevelParseError};

#[cfg(feature = "env-filter")]
#[cfg_attr(docsrs, doc(cfg(feature = "env-filter")))]
pub use self::env::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TargetFilter(Option<Arc<str>>);

impl TargetFilter {
    pub(crate) fn matches(&self, target: &str) -> bool {
        if let Some(filter) = &self.0 {
            target.starts_with(&filter[..])
        } else {
            true
        }
    }

    pub(crate) fn len(&self) -> Option<usize> {
        self.0.as_ref().map(|s| s[..].len())
    }
}

impl<'a> From<Option<&'a str>> for TargetFilter {
    fn from(t: Option<&'a str>) -> TargetFilter {
        TargetFilter(t.map(Arc::from))
    }
}

impl AsRef<Option<Arc<str>>> for TargetFilter {
    fn as_ref(&self) -> &Option<Arc<str>> {
        &self.0
    }
}
