pub use tokio_trace_core::callsite::{register, Callsite};

#[cfg(any(test, feature = "test-support"))]
pub use tokio_trace_core::callsite::reset_registry;
