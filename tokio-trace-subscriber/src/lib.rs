//! Utilities and helpers for implementing and composing subscribers.

#[cfg(feature = "store")]
extern crate owning_ref;
#[cfg(feature = "store")]
extern crate parking_lot;

extern crate tokio_trace_core;

pub mod observe;
pub mod span;

use std::{cell::RefCell, default::Default, thread};

/// Tracks the currently executing span on a per-thread basis.
#[derive(Clone)]
pub struct CurrentSpanPerThread {
    current: &'static thread::LocalKey<RefCell<Vec<span::Id>>>,
}

impl CurrentSpanPerThread {
    pub fn new() -> Self {
        thread_local! {
            static CURRENT: RefCell<Vec<span::Id>> = RefCell::new(vec![]);
        };
        Self { current: &CURRENT }
    }

    /// Returns the [`d`](::span::Id) of the span in which the current thread is
    /// executing, or `None` if it is not inside of a span.
    pub fn id(&self) -> Option<span::Id> {
        self.current
            .with(|current| current.borrow().last().cloned())
    }

    pub fn enter(&self, span: span::Id) {
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
