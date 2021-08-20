//! [`Layer`]s that control which spans and events are enabled by the wrapped
//! subscriber.
//!
//! [`Layer`]: ../layer/trait.Layer.html
#[cfg(feature = "env-filter")]
mod env;
mod level;

pub use self::level::{LevelFilter, ParseError as LevelParseError};

#[cfg(feature = "env-filter")]
#[cfg_attr(docsrs, doc(cfg(feature = "env-filter")))]
pub use self::env::*;

use crate::layer::Context;
use std::num::NonZeroU64;
use tracing_core::{subscriber::Interest, Metadata};

/// A filter that determines whether a span or event is enabled.
pub trait Filter<S> {
    fn enabled(&self, meta: &Metadata<'_>, cx: Context<'_, S>) -> bool;
    fn callsite_enabled(&self, meta: &'static Metadata<'static>, cx: Context<'_, S>) -> Interest;
}

#[derive(Copy, Clone, Debug)]
pub struct FilterId(NonZeroU64);
