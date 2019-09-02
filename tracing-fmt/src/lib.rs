//! A `Subscriber` for formatting and logging `tracing` data.
//!
//! **Note**: This library is now part of the [`tracing-subscriber`] crate. This
//! crate now re-exports its public API from `tracing-subscriber`. Using
//! `tracing-fmt` is now deprecated; users are encouraged to use the APIs in
//! this library from their new home in `tracing-subscriber::fmt`.
//!
//! ## Overview
//!
//! [`tracing`] is a framework for instrumenting Rust programs with context-aware,
//! structured, event-based diagnostic information. This crate provides an
//! implementation of the [`Subscriber`] trait that records `tracing`'s `Event`s
//! and `Span`s by formatting them as text and logging them to stdout.
//!
//!
//! [`tracing`]: https://crates.io/crates/tracing
//! [`Subscriber`]: https://docs.rs/tracing/latest/tracing/trait.Subscriber.html
//! [`tracing-subscriber`]: https://crates.io/crates/tracing-subscriber/
#![doc(html_root_url = "https://docs.rs/tracing-fmt/0.0.1")]
#![cfg_attr(test, deny(warnings))]
#![deprecated(since = "0.0.1", note = "moved to `tracing-subscriber::fmt`")]

#[deprecated(since = "0.0.1", note = "moved to `tracing-subscriber::fmt`")]
#[doc(inline)]
pub use crate::{format::FormatEvent, writer::MakeWriter};

#[deprecated(since = "0.0.1", note = "moved to `tracing-subscriber::fmt`")]
#[doc(inline)]
pub use tracing_subscriber::{fmt::Builder, fmt::Context, FmtSubscriber};

#[deprecated(since = "0.0.1", note = "moved to `tracing-subscriber::fmt::format`")]
pub mod format {
    #[doc(inline)]
    pub use tracing_subscriber::fmt::format::*;
}

#[deprecated(since = "0.0.1", note = "moved to `tracing-subscriber::fmt::writer`")]
pub mod writer {
    #[doc(inline)]
    pub use tracing_subscriber::fmt::writer::*;
}

#[deprecated(since = "0.0.1", note = "moved to `tracing-subscriber::fmt::time`")]
pub mod time {
    #[doc(inline)]
    pub use tracing_subscriber::fmt::time::*;
}

#[deprecated(since = "0.0.1", note = "moved to `tracing-subscriber::filter`")]
pub mod filter {
    #[doc(inline)]
    pub use tracing_subscriber::Filter as EnvFilter;
}
