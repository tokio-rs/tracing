//! The `tracing-subscriber` prelude.
//!
//! This brings into scope a number of extension traits that define methods on
//! types defined here and in other crates.

// apparently `as _` reexports now generate `unreachable_pub` linting? which
// seems wrong to me...
#![allow(unreachable_pub)]
pub use crate::field::{MakeExt as _, RecordFields as _};
pub use crate::subscribe::{CollectExt as _, Subscribe as _};
pub use crate::util::SubscriberInitExt as _;

feature! {
    #![all(feature = "fmt", feature = "std")]
    pub use crate::fmt::writer::MakeWriterExt as _;
}
