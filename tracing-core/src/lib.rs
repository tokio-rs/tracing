//! Core primitives for `tracing`.
//!
//! [`tracing`] is a framework for instrumenting Rust programs to collect
//! structured, event-based diagnostic information. This crate defines the core
//! primitives of `tracing`.
//!
//! This crate provides:
//!
//! * [`span::Id`] identifies a span within the execution of a program.
//!
//! * [`Event`] represents a single event within a trace.
//!
//! * [`Collector`], the trait implemented to collect trace data.
//!
//! * [`Metadata`] and [`Callsite`] provide information describing spans and
//!   `Event`s.
//!
//! * [`Field`], [`FieldSet`], [`Value`], and [`ValueSet`] represent the
//!   structured data attached to a span.
//!
//! * [`Dispatch`] allows spans and events to be dispatched to `Collector`s.
//!
//! In addition, it defines the global callsite registry and per-thread current
//! dispatcher which other components of the tracing system rely on.
//!
//! *Compiler support: [requires `rustc` 1.42+][msrv]*
//!
//! [msrv]: #supported-rust-versions
//!
//! ## Usage
//!
//! Application authors will typically not use this crate directly. Instead,
//! they will use the [`tracing`] crate, which provides a much more
//! fully-featured API. However, this crate's API will change very infrequently,
//! so it may be used when dependencies must be very stable.
//!
//! `Collector` implementations may depend on `tracing-core` rather than
//! `tracing`, as the additional APIs provided by `tracing` are primarily useful
//! for instrumenting libraries and applications, and are generally not
//! necessary for `Collector` implementations.
//!
//! The [`tokio-rs/tracing`] repository contains less stable crates designed to
//! be used with the `tracing` ecosystem. It includes a collection of
//! `Collector` implementations, as well as utility and adapter crates.
//!
//! ### `no_std` Support
//!
//! In embedded systems and other bare-metal applications, `tracing-core` can be
//! used without requiring the Rust standard library, although some features are
//! disabled.
//!
//! The dependency on the standard library is controlled by two crate feature
//! flags, "std", which enables the dependency on [`libstd`], and "alloc", which
//! enables the dependency on [`liballoc`] (and is enabled by the "std"
//! feature). These features are enabled by default, but `no_std` users can
//! disable them using:
//!
//! ```toml
//! # Cargo.toml
//! tracing-core = { version = "0.2", default-features = false }
//! ```
//!
//! To enable `liballoc` but not `std`, use:
//!
//! ```toml
//! # Cargo.toml
//! tracing-core = { version = "0.2", default-features = false, features = ["alloc"] }
//! ```
//!
//! When both the "std" and "alloc" feature flags are disabled, `tracing-core`
//! will not make any dynamic memory allocations at runtime, and does not
//! require a global memory allocator.
//!
//! The "alloc" feature is required to enable the [`Dispatch::new`] function,
//! which requires dynamic memory allocation to construct a `Collector` trait
//! object at runtime. When liballoc is disabled, new `Dispatch`s may still be
//! created from `&'static dyn Collector` references, using
//! [`Dispatch::from_static`].
//!
//! The "std" feature is required to enable the following features:
//!
//! * Per-thread scoped trace dispatchers ([`Dispatch::set_default`] and
//!   [`with_default`]. Since setting a thread-local dispatcher inherently
//!   requires a concept of threads to be available, this API is not possible
//!   without the standard library.
//! * Support for [constructing `Value`s from types implementing
//!   `std::error::Error`][err]. Since the `Error` trait is defined in `std`,
//!   it's not possible to provide this feature without `std`.
//!
//! All other features of `tracing-core` should behave identically with and
//! without `std` and `alloc`.
//!
//! [`libstd`]: https://doc.rust-lang.org/std/index.html
//! [`Dispatch::new`]: crate::dispatcher::Dispatch::new
//! [`Dispatch::from_static`]: crate::dispatcher::Dispatch::from_static
//! [`Dispatch::set_default`]: crate::dispatcher::set_default
//! [`with_default`]: crate::dispatcher::with_default
//! [err]: crate::field::Visit::record_error
//!
//! ### Crate Feature Flags
//!
//! The following crate feature flags are available:
//!
//! * `std`: Depend on the Rust standard library (enabled by default).
//! * `alloc`: Depend on [`liballoc`] (enabled by "std").
//!
//! [`liballoc`]: https://doc.rust-lang.org/alloc/index.html
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
//!
//! [`span::Id`]: span::Id
//! [`Event`]: event::Event
//! [`Collector`]: collector::Collector
//! [`Metadata`]: metadata::Metadata
//! [`Callsite`]: callsite::Callsite
//! [`Field`]: field::Field
//! [`FieldSet`]: field::FieldSet
//! [`Value`]: field::Value
//! [`ValueSet`]: field::ValueSet
//! [`Dispatch`]: dispatcher::Dispatch
//! [`tokio-rs/tracing`]: https://github.com/tokio-rs/tracing
//! [`tracing`]: https://crates.io/crates/tracing
#![doc(html_root_url = "https://docs.rs/tracing-core/0.1.17")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/logo-type.png",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]
#![cfg_attr(not(feature = "std"), no_std)]
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

#[cfg(feature = "alloc")]
extern crate alloc;

/// Statically constructs an [`Identifier`] for the provided [`Callsite`].
///
/// This may be used in contexts, such as static initializers, where the
/// [`Metadata::callsite`] function is not currently usable.
///
/// For example:
/// ```rust
/// # #[macro_use]
/// # extern crate tracing_core;
/// use tracing_core::callsite;
/// # use tracing_core::{Metadata, collector::Interest};
/// # fn main() {
/// pub struct MyCallsite {
///    // ...
/// }
/// impl callsite::Callsite for MyCallsite {
/// # fn set_interest(&self, _: Interest) { unimplemented!() }
/// # fn metadata(&self) -> &Metadata { unimplemented!() }
///     // ...
/// }
///
/// static CALLSITE: MyCallsite = MyCallsite {
///     // ...
/// };
///
/// static CALLSITE_ID: callsite::Identifier = identify_callsite!(&CALLSITE);
/// # }
/// ```
///
/// [`Identifier`]: callsite::Identifier
/// [`Callsite`]: callsite::Callsite
/// [`Metadata::callsite`]: metadata::Metadata::callsite
#[macro_export]
macro_rules! identify_callsite {
    ($callsite:expr) => {
        $crate::callsite::Identifier($callsite)
    };
}

/// Statically constructs new span [metadata].
///
/// /// For example:
/// ```rust
/// # #[macro_use]
/// # extern crate tracing_core;
/// # use tracing_core::{callsite::Callsite, collector::Interest};
/// use tracing_core::metadata::{Kind, Level, Metadata};
/// # fn main() {
/// # pub struct MyCallsite { }
/// # impl Callsite for MyCallsite {
/// # fn set_interest(&self, _: Interest) { unimplemented!() }
/// # fn metadata(&self) -> &Metadata { unimplemented!() }
/// # }
/// #
/// static FOO_CALLSITE: MyCallsite = MyCallsite {
///     // ...
/// };
///
/// static FOO_METADATA: Metadata = metadata!{
///     name: "foo",
///     target: module_path!(),
///     level: Level::DEBUG,
///     fields: &["bar", "baz"],
///     callsite: &FOO_CALLSITE,
///     kind: Kind::SPAN,
/// };
/// # }
/// ```
///
/// [metadata]: metadata::Metadata
/// [`Metadata::new`]: metadata::Metadata::new
#[macro_export]
macro_rules! metadata {
    (
        name: $name:expr,
        target: $target:expr,
        level: $level:expr,
        fields: $fields:expr,
        callsite: $callsite:expr,
        kind: $kind:expr
    ) => {
        $crate::metadata! {
            name: $name,
            target: $target,
            level: $level,
            fields: $fields,
            callsite: $callsite,
            kind: $kind,
        }
    };
    (
        name: $name:expr,
        target: $target:expr,
        level: $level:expr,
        fields: $fields:expr,
        callsite: $callsite:expr,
        kind: $kind:expr,
    ) => {
        $crate::metadata::Metadata::new(
            $name,
            $target,
            $level,
            Some(file!()),
            Some(line!()),
            Some(module_path!()),
            $crate::field::FieldSet::new($fields, $crate::identify_callsite!($callsite)),
            $kind,
        )
    };
}

// std uses lazy_static from crates.io
#[cfg(feature = "std")]
#[macro_use]
extern crate lazy_static;

// Facade module: `no_std` uses spinlocks, `std` uses the mutexes in the standard library
#[cfg(not(feature = "std"))]
#[doc(hidden)]
pub type Once = crate::spin::Once<()>;

#[cfg(feature = "std")]
#[doc(hidden)]
pub use std::sync::Once;

#[cfg(not(feature = "std"))]
// Trimmed-down vendored version of spin 0.5.2 (0387621)
// Required for `Once` in `no_std` builds.
pub(crate) mod spin;

pub mod callsite;
pub mod collector;
pub mod dispatcher;
pub mod event;
pub mod field;
pub mod metadata;
mod parent;
pub mod span;

#[doc(inline)]
pub use self::{
    callsite::Callsite,
    collector::Collector,
    dispatcher::Dispatch,
    event::Event,
    field::Field,
    metadata::{Level, LevelFilter, Metadata},
};

pub use self::{collector::Interest, metadata::Kind};

mod sealed {
    pub trait Sealed {}
}
