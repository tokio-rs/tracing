#![doc = include_str!("../README.md")]
#![cfg_attr(
    docsrs,
    // Allows displaying cfgs/feature flags in the documentation.
    feature(doc_cfg),
    // Fail the docs build if any intra-docs links are broken
    deny(rustdoc::broken_intra_doc_links),
)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/logo-type.png",
    html_favicon_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/favicon.ico",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]
#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub,
    bad_style,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_interfaces,
    private_bounds,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true
)]

pub mod ancestry;
pub mod collector;
pub mod event;
pub mod expect;
pub mod field;
mod metadata;
pub mod span;

#[cfg(feature = "tracing-subscriber")]
pub mod subscriber;
