//! Dispatches trace events to a [`Collect`].
//!
//! The _dispatcher_ is the component of the tracing system which is responsible
//! for forwarding trace data from the instrumentation points that generate it
//! to the collector that collects it.
//!
//! # Using the Trace Dispatcher
//!
//! Every thread in a program using `tracing` has a _default collector_. When
//! events occur, or spans are created, they are dispatched to the thread's
//! current collector.
//!
//! ## Setting the Default Collector
//!
//! By default, the current collector is an empty implementation that does
//! nothing. To use a collector implementation, it must be set as the default.
//! There are two methods for doing so: [`with_default`] and
//! [`set_global_default`]. `with_default` sets the default collector for the
//! duration of a scope, while `set_global_default` sets a default collector
//! for the entire process.
//!
//! To use either of these functions, we must first wrap our collector in a
//! [`Dispatch`], a cloneable, type-erased reference to a collector. For
//! example:
//! ```rust
//! # pub struct FooCollector;
//! # use tracing_core::{
//! #   dispatch, Event, Metadata,
//! #   span::{Attributes, Id, Record}
//! # };
//! # impl tracing_core::Collect for FooCollector {
//! #   fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(0) }
//! #   fn record(&self, _: &Id, _: &Record) {}
//! #   fn event(&self, _: &Event) {}
//! #   fn record_follows_from(&self, _: &Id, _: &Id) {}
//! #   fn enabled(&self, _: &Metadata) -> bool { false }
//! #   fn enter(&self, _: &Id) {}
//! #   fn exit(&self, _: &Id) {}
//! # }
//! # impl FooCollector { fn new() -> Self { FooCollector } }
//! # #[cfg(feature = "alloc")]
//! use dispatch::Dispatch;
//!
//! # #[cfg(feature = "alloc")]
//! let my_collector = FooCollector::new();
//! # #[cfg(feature = "alloc")]
//! let my_dispatch = Dispatch::new(my_collector);
//! ```
//! Then, we can use [`with_default`] to set our `Dispatch` as the default for
//! the duration of a block:
//! ```rust
//! # pub struct FooCollector;
//! # use tracing_core::{
//! #   dispatch, Event, Metadata,
//! #   span::{Attributes, Id, Record}
//! # };
//! # impl tracing_core::Collect for FooCollector {
//! #   fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(0) }
//! #   fn record(&self, _: &Id, _: &Record) {}
//! #   fn event(&self, _: &Event) {}
//! #   fn record_follows_from(&self, _: &Id, _: &Id) {}
//! #   fn enabled(&self, _: &Metadata) -> bool { false }
//! #   fn enter(&self, _: &Id) {}
//! #   fn exit(&self, _: &Id) {}
//! # }
//! # impl FooCollector { fn new() -> Self { FooCollector } }
//! # #[cfg(feature = "alloc")]
//! # let my_collector = FooCollector::new();
//! # #[cfg(feature = "alloc")]
//! # let my_dispatch = dispatch::Dispatch::new(my_collector);
//! // no default collector
//!
//! # #[cfg(feature = "std")]
//! dispatch::with_default(&my_dispatch, || {
//!     // my_collector is the default
//! });
//!
//! // no default collector again
//! ```
//! It's important to note that `with_default` will not propagate the current
//! thread's default collector to any threads spawned within the `with_default`
//! block. To propagate the default collector to new threads, either use
//! `with_default` from the new thread, or use `set_global_default`.
//!
//! As an alternative to `with_default`, we can use [`set_global_default`] to
//! set a `Dispatch` as the default for all threads, for the lifetime of the
//! program. For example:
//! ```rust
//! # pub struct FooCollector;
//! # use tracing_core::{
//! #   dispatch, Event, Metadata,
//! #   span::{Attributes, Id, Record}
//! # };
//! # impl tracing_core::Collect for FooCollector {
//! #   fn new_span(&self, _: &Attributes) -> Id { Id::from_u64(0) }
//! #   fn record(&self, _: &Id, _: &Record) {}
//! #   fn event(&self, _: &Event) {}
//! #   fn record_follows_from(&self, _: &Id, _: &Id) {}
//! #   fn enabled(&self, _: &Metadata) -> bool { false }
//! #   fn enter(&self, _: &Id) {}
//! #   fn exit(&self, _: &Id) {}
//! # }
//! # impl FooCollector { fn new() -> Self { FooCollector } }
//! # #[cfg(feature = "alloc")]
//! # let my_collector = FooCollector::new();
//! # #[cfg(feature = "alloc")]
//! # let my_dispatch = dispatch::Dispatch::new(my_collector);
//! // no default collector
//!
//! # #[cfg(feature = "alloc")]
//! dispatch::set_global_default(my_dispatch)
//!     // `set_global_default` will return an error if the global default
//!     // collector has already been set.
//!     .expect("global default was already set!");
//!
//! // `my_collector` is now the default
//! ```
//! <div class="information">
//!     <div class="tooltip ignore" style="">ⓘ<span class="tooltiptext">Note</span></div>
//! </div>
//! <div class="example-wrap" style="display:inline-block">
//! <pre class="ignore" style="white-space:normal;font:inherit;">
//! <strong>Note</strong>: The thread-local scoped dispatcher (<code>with_default</code>)
//! requires the Rust standard library. <code>no_std</code> users should
//! use <a href="fn.set_global_default.html"><code>set_global_default</code></a>
//! instead.
//! </pre></div>
//!
//! ## Accessing the Default Collector
//!
//! A thread's current default collector can be accessed using the
//! [`get_default`] function, which executes a closure with a reference to the
//! currently default `Dispatch`. This is used primarily by `tracing`
//! instrumentation.
//!
//! [`Collect`]: tracing_core::Collect
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub use tracing_core::dispatch::set_default;
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub use tracing_core::dispatch::with_default;
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub use tracing_core::dispatch::DefaultGuard;
pub use tracing_core::dispatch::{
    get_default, set_global_default, Dispatch, SetGlobalDefaultError,
};

/// Private API for internal use by tracing's macros.
///
/// This function is *not* considered part of `tracing`'s public API, and has no
/// stability guarantees. If you use it, and it breaks or disappears entirely,
/// don't say we didn;'t warn you.
#[doc(hidden)]
pub use tracing_core::dispatch::has_been_set;
