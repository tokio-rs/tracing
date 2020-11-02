//! Collects and records trace data.
pub use tracing_core::collect::*;

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub use tracing_core::dispatch::DefaultGuard;

/// Sets this collector as the default for the duration of a closure.
///
/// The default collector is used when creating a new [`Span`] or
/// [`Event`], _if no span is currently executing_. If a span is currently
/// executing, new spans or events are dispatched to the collector that
/// tagged that span, instead.
///
/// [`Span`]: super::span::Span
/// [`Collect`]: super::collect::Collect
/// [`Event`]: :../event/struct.Event.html
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub fn with_default<T, S>(collector: S, f: impl FnOnce() -> T) -> T
where
    S: Collect + Send + Sync + 'static,
{
    crate::dispatch::with_default(&crate::Dispatch::new(collector), f)
}

/// Sets this collector as the global default for the duration of the entire program.
/// Will be used as a fallback if no thread-local collector has been set in a thread (using `with_default`.)
///
/// Can only be set once; subsequent attempts to set the global default will fail.
/// Returns whether the initialization was successful.
///
/// Note: Libraries should *NOT* call `set_global_default()`! That will cause conflicts when
/// executables try to set them later.
///
/// [span]: super::span
/// [`Event`]: super::event::Event
#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
pub fn set_global_default<S>(collector: S) -> Result<(), SetGlobalDefaultError>
where
    S: Collect + Send + Sync + 'static,
{
    crate::dispatch::set_global_default(crate::Dispatch::new(collector))
}

/// Sets the collector as the default for the duration of the lifetime of the
/// returned [`DefaultGuard`]
///
/// The default collector is used when creating a new [`Span`] or
/// [`Event`], _if no span is currently executing_. If a span is currently
/// executing, new spans or events are dispatched to the collector that
/// tagged that span, instead.
///
/// [`Span`]: super::span::Span
/// [`Event`]: :../event/struct.Event.html
/// [`DefaultGuard`]: super::dispatcher::DefaultGuard
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[must_use = "Dropping the guard unregisters the collector."]
pub fn set_default<S>(collector: S) -> DefaultGuard
where
    S: Collect + Send + Sync + 'static,
{
    crate::dispatch::set_default(&crate::Dispatch::new(collector))
}

pub use tracing_core::dispatch::SetGlobalDefaultError;
