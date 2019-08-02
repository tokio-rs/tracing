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
pub mod layer;
pub mod prelude;
pub mod reload;

pub(crate) mod thread;
pub use layer::Layer;
use std::default::Default;

pub type CurrentSpanPerThread = CurrentSpan;

/// Tracks the currently executing span on a per-thread basis.
pub struct CurrentSpan {
    current: thread::Local<Vec<Id>>,
}

impl CurrentSpan {
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

    pub fn enter(&self, span: Id) {
        self.current.get().push(span)
    }

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
    pub struct SealedTy {
        _p: (),
    }
    pub trait Sealed<A = SealedTy> {}
}
