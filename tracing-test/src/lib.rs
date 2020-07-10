//! Testing utilities for `tracing`
//!
//! # Overview
//!
//! `tracing-test` offers mock implementations of useful `tracing` types to enable easier unit
//! testing of `tracing`-based crates.
//!
//! # Usage
//!
//! Add the following to your `Cargo.toml`:
//! ```toml
//! tracing-test = "0.1"
//! ```
//!
//! ## Mock subscriber
//!
//! ```rust
//! # fn docs() {
//! // TODO
//! # }
//! ```
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(html_root_url = "https://docs.rs/tracing-test/0.1.0")]
#![warn(
    // missing_docs,
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

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod event;
pub mod field;
pub mod metadata;
pub mod span;
pub mod subscriber;

pub use span::mock as span;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Parent {
    ContextualRoot,
    Contextual(String),
    ExplicitRoot,
    Explicit(String),
}
