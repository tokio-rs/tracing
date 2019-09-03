//! Utilities for implementing and composing [`tracing`] subscribers.
//!
//! [`tracing`] is a framework for instrumenting Rust programs to collect
//! scoped, structured, and async-aware diagnostics. The [`Subscriber`] trait
//! represents the functionality necessary to collect this trace data. This
//! crate contains tools for composing subscribers out of smaller units of
//! behaviour, and batteries-included implementations of common subscriber
//! functionality.
//!
//! `tracing-subscriber` is intended for use by both `Subscriber` authors and
//! application authors using `tracing` to instrument their applications.
//!
//! [`tracing`]: https://docs.rs/tracing/latest/tracing/
//! [`Subscriber`]: https://docs.rs/tracing-core/latest/tracing_core/subscriber/trait.Subscriber.html
#![doc(html_root_url = "https://docs.rs/tracing-subscriber/0.0.1-alpha.4")]
#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub
)]
#![cfg_attr(
    test,
    deny(
        missing_debug_implementations,
        missing_docs,
        rust_2018_idioms,
        unreachable_pub,
        bad_style,
        const_err,
        dead_code,
        improper_ctypes,
        legacy_directory_ownership,
        non_shorthand_field_patterns,
        no_mangle_generic_items,
        overflowing_literals,
        path_statements,
        patterns_in_fns_without_body,
        plugin_as_library,
        private_in_public,
        safe_extern_statics,
        unconditional_recursion,
        unions_with_drop_fields,
        unused,
        unused_allocation,
        unused_comparisons,
        unused_parens,
        while_true
    )
)]
use tracing_core::span::Id;

#[macro_use]
macro_rules! try_lock {
    ($lock:expr) => {
        try_lock!($lock, else return)
    };
    ($lock:expr, else $els:expr) => {
        match $lock {
            Ok(l) => l,
            Err(_) if std::thread::panicking() => $els,
            Err(_) => panic!("lock poisoned"),
        }
    };
}

#[cfg(feature = "filter")]
pub mod filter;
#[cfg(feature = "fmt")]
pub mod fmt;
pub mod layer;
pub mod prelude;
pub mod reload;
pub(crate) mod thread;

#[cfg(feature = "filter")]
pub use filter::Filter;
pub use layer::Layer;

#[cfg(feature = "fmt")]
pub use fmt::Subscriber as FmtSubscriber;

use std::default::Default;

/// Tracks the currently executing span on a per-thread basis.
#[deprecated(since = "0.0.1-alpha.2", note = "renamed to `CurrentSpan`")]
pub type CurrentSpanPerThread = CurrentSpan;

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

    /// Returns the [`Id`](::Id) of the span in which the current thread is
    /// executing, or `None` if it is not inside of a span.
    pub fn id(&self) -> Option<Id> {
        self.current.get().last().cloned()
    }

    /// Records that the current thread has entered the span with the provided ID.
    pub fn enter(&self, span: Id) {
        self.current.get().push(span)
    }

    /// Records that the current thread has exited a span.
    pub fn exit(&self) {
        self.current.get().pop();
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
