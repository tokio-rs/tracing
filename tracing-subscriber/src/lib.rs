//! Utilities and helpers for implementing and composing subscribers.
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
