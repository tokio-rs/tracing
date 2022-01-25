#![allow(dead_code)]
//! This example shows how a field value may be recorded using the `valuable`
//! crate (https://crates.io/crates/valuable).
//!
//! `valuable` provides a lightweight but flexible way to record structured data, allowing
//! visitors to extract individual fields or elements of structs, maps, arrays, and other
//! nested structures.
//!
//! `tracing`'s support for `valuable` is currently feature flagged. Additionally, `valuable`
//! support is considered an *unstable feature*: in order to use `valuable` with `tracing`,
//! the project must be built with `RUSTFLAGS="--cfg tracing_unstable`.
//!
//! Therefore, when `valuable` support is not enabled, this example falls back to using
//! `fmt::Debug` to record fields that implement `valuable::Valuable`.
#[cfg(tracing_unstable)]
use tracing::field::valuable;
use valuable::Valuable;

#[derive(Clone, Debug, Valuable)]
struct User {
    name: String,
    age: u32,
    address: Address,
}

#[derive(Clone, Debug, Valuable)]
struct Address {
    country: String,
    city: String,
    street: String,
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .json()
        .init();

    let user = User {
        name: "Arwen Undomiel".to_string(),
        age: 3000,
        address: Address {
            country: "Middle Earth".to_string(),
            city: "Rivendell".to_string(),
            street: "leafy lane".to_string(),
        },
    };

    // for comparison, record `user` without using its `Valuable`
    // implementation:
    tracing::info!(valuable = false, user = ?user);

    // If the `valuable` feature is enabled, record `user` using its'
    // `valuable::Valuable` implementation:
    #[cfg(tracing_unstable)]
    tracing::info!(valuable = true, user = valuable(&user));

    #[cfg(not(tracing_unstable))]
    tracing::warn!(
        "note: this example was run without `valuable` support enabled!\n\
        rerun with `RUSTFLAGS=\"--cfg tracing_unstable\" to enable `valuable`",
    );
}
