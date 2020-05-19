//! The `tracing` prelude.
//!
//! This brings into scope the most commonly used `tracing` macros and structs.
//! You'll almost always want to import the prelude's entire contents:
//!
//! ```
//! # #![allow(warnings)]
//! use tracing::prelude::*;
//! ```

pub use crate::{
    debug, debug_span, error, error_span, event, info, info_span, span, trace, trace_span, warn,
    warn_span, Level,
};
