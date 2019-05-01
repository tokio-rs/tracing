//! Utilities and helpers for implementing and composing subscribers.

extern crate tokio_trace;
pub use tokio_trace::{Event, Id};

// mod compose;
// pub use compose::Composed;

// pub mod filter;
// pub mod observe;
// pub mod registry;

// pub use filter::{Filter, FilterExt};
// pub use observe::{Observe, ObserveExt};
// pub use registry::{RegisterSpan, SpanRef};

use std::{cell::RefCell, default::Default, thread};

/// Tracks the currently executing span on a per-thread basis.
#[derive(Clone)]
pub struct CurrentSpanPerThread {
    current: &'static thread::LocalKey<RefCell<Vec<Id>>>,
}

impl CurrentSpanPerThread {
    pub fn new() -> Self {
        thread_local! {
            static CURRENT: RefCell<Vec<Id>> = RefCell::new(vec![]);
        };
        Self { current: &CURRENT }
    }

    /// Returns the [`Id`](::Id) of the span in which the current thread is
    /// executing, or `None` if it is not inside of a span.
    pub fn id(&self) -> Option<Id> {
        self.current
            .with(|current| current.borrow().last().cloned())
    }

    pub fn enter(&self, span: Id) {
        self.current.with(|current| {
            current.borrow_mut().push(span);
        })
    }

    pub fn exit(&self) {
        self.current.with(|current| {
            let _ = current.borrow_mut().pop();
        })
    }
}

impl Default for CurrentSpanPerThread {
    fn default() -> Self {
        Self::new()
    }
}
