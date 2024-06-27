//! The `tracing` prelude.
//!
//! This brings into scope a number of extension traits that define methods on
//! types defined here and in other crates.

pub use crate::field;
pub use crate::{
    debug, debug_span, error, error_span, event, info, info_span, span, trace, trace_span, warn,
    warn_span, Level,
};
