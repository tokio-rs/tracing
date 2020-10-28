//! Utilities for implementing and composing [`tracing`] subscribers.
//!
//! [`tracing`] is a framework for instrumenting Rust programs to collect
//! scoped, structured, and async-aware diagnostics. The [`Collect`] trait
//! represents the functionality necessary to collect this trace data. This
//! crate contains tools for composing subscribers out of smaller units of
//! behaviour, and batteries-included implementations of common subscriber
//! functionality.
//!
//! `tracing-subscriber` is intended for use by both `Collector` authors and
//! application authors using `tracing` to instrument their applications.
//!
//! *Compiler support: [requires `rustc` 1.42+][msrv]*
//!
//! [msrv]: #supported-rust-versions
//!
//! ## Included Subscribers
//!
//! The following `Collector`s are provided for application authors:
//!
//! - [`fmt`] - Formats and logs tracing data (requires the `fmt` feature flag)
//!
//! ## Feature Flags
//!
//! - `env-filter`: Enables the [`EnvFilter`] type, which implements filtering
//!   similar to the [`env_logger` crate]. Enabled by default.
//! - `fmt`: Enables the [`fmt`] module, which provides a subscriber
//!   implementation for printing formatted representations of trace events.
//!   Enabled by default.
//! - `ansi`: Enables `fmt` support for ANSI terminal colors. Enabled by
//!   default.
//! - `registry`: enables the [`registry`] module. Enabled by default.
//! - `json`: Enables `fmt` support for JSON output. In JSON output, the ANSI feature does nothing.
//!
//! ### Optional Dependencies
//!
//! - [`tracing-log`]: Enables better formatting for events emitted by `log`
//!   macros in the `fmt` subscriber. On by default.
//! - [`chrono`]: Enables human-readable time formatting in the `fmt` subscriber.
//!   Enabled by default.
//! - [`smallvec`]: Causes the `EnvFilter` type to use the `smallvec` crate (rather
//!   than `Vec`) as a performance optimization. Enabled by default.
//! - [`parking_lot`]: Use the `parking_lot` crate's `RwLock` implementation
//!   rather than the Rust standard library's implementation.
//!
//! ## Supported Rust Versions
//!
//! Tracing is built against the latest stable release. The minimum supported
//! version is 1.42. The current Tracing version is not guaranteed to build on
//! Rust versions earlier than the minimum supported version.
//!
//! Tracing follows the same compiler support policies as the rest of the Tokio
//! project. The current stable Rust compiler and the three most recent minor
//! versions before it will always be supported. For example, if the current
//! stable compiler version is 1.45, the minimum supported version will not be
//! increased past 1.42, three minor versions prior. Increasing the minimum
//! supported compiler version is not considered a semver breaking change as
//! long as doing so complies with this policy.
//!
//! [`tracing`]: https://docs.rs/tracing/latest/tracing/
//! [`Collect`]: https://docs.rs/tracing-core/latest/tracing_core/collect/trait.Collect.html
//! [`EnvFilter`]: filter/struct.EnvFilter.html
//! [`fmt`]: fmt/index.html
//! [`tracing-log`]: https://crates.io/crates/tracing-log
//! [`smallvec`]: https://crates.io/crates/smallvec
//! [`chrono`]: https://crates.io/crates/chrono
//! [`env_logger` crate]: https://crates.io/crates/env_logger
//! [`parking_lot`]: https://crates.io/crates/parking_lot
//! [`registry`]: registry/index.html
#![doc(html_root_url = "https://docs.rs/tracing-subscriber/0.2.12")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/logo-type.png",
    html_favicon_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/favicon.ico",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]
#![cfg_attr(docsrs, feature(doc_cfg), deny(broken_intra_doc_links))]
#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub,
    bad_style,
    const_err,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_in_public,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true
)]
// Using struct update syntax when a struct has no additional fields avoids
// a potential source change if additional fields are added to the struct in the
// future, reducing diff noise. Allow this even though clippy considers it
// "needless".
#![allow(clippy::needless_update)]

use tracing_core::span::Id;

#[macro_use]
mod macros;

pub mod field;
pub mod filter;
#[cfg(feature = "fmt")]
#[cfg_attr(docsrs, doc(cfg(feature = "fmt")))]
pub mod fmt;
pub mod prelude;
pub mod registry;
pub mod reload;
pub mod subscribe;
pub(crate) mod sync;
pub(crate) mod thread;
pub mod util;

#[cfg(feature = "env-filter")]
#[cfg_attr(docsrs, doc(cfg(feature = "env-filter")))]
pub use filter::EnvFilter;

pub use subscribe::Subscribe;

cfg_feature!("fmt", {
    pub use fmt::fmt;
    pub use fmt::Subscriber as FmtSubscriber;
});

cfg_feature!("registry", {
    pub use registry::Registry;

    ///
    pub fn registry() -> Registry {
        Registry::default()
    }
});

use std::default::Default;
/// Tracks the currently executing span on a per-thread basis.
#[derive(Debug)]
pub struct CurrentSpan {
    current: thread::Local<Vec<Id>>,
}

impl CurrentSpan {
    /// Returns a new `CurrentSpan`.
    pub fn new() -> Self {
        Self {
            current: thread::Local::new(),
        }
    }

    /// Returns the [`Id`] of the span in which the current thread is
    /// executing, or `None` if it is not inside of a span.
    ///
    ///
    /// [`Id`]: https://docs.rs/tracing/latest/tracing/span/struct.Id.html
    pub fn id(&self) -> Option<Id> {
        self.current.with(|current| current.last().cloned())?
    }

    /// Records that the current thread has entered the span with the provided ID.
    pub fn enter(&self, span: Id) {
        self.current.with(|current| current.push(span));
    }

    /// Records that the current thread has exited a span.
    pub fn exit(&self) {
        self.current.with(|current| {
            let _ = current.pop();
        });
    }
}

impl Default for CurrentSpan {
    fn default() -> Self {
        Self::new()
    }
}

mod sealed {
    pub trait Sealed<A = ()> {}
}
