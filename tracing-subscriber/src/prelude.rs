//! The `tracing-subscriber` prelude.
//!
//! This brings into scope a number of extension traits that define methods on
//! types defined here and in other crates.

pub use crate::field::{MakeExt as _, RecordFields as _};
pub use crate::subscribe::{CollectExt as _, Subscribe as _};
pub use crate::util::SubscriberInitExt as _;

#[cfg(feature = "fmt")]
#[cfg_attr(docsrs, doc(cfg(feature = "fmt")))]
pub use crate::fmt::writer::MakeWriterExt as _;
