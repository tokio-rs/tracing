//! The `tracing-subscriber` prelude.
//!
//! This brings into scope a number of extension traits that define methods on
//! types defined here and in other crates.

pub use crate::field::{MakeExt as _, RecordFields as _};
pub use crate::layer::{Layer as _, SubscriberExt as _};
pub use crate::util::SubscriberInitExt as _;

feature! {
    #![all(feature = "fmt", feature = "std")]
    pub use crate::fmt::writer::MakeWriterExt as _;
}
