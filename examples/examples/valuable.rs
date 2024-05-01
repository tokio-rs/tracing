#![allow(dead_code)]
//! This example shows how a field value may be recorded using the `valuable`
//! crate (https://crates.io/crates/valuable).
//!
//! `valuable` provides a lightweight but flexible way to record structured data, allowing
//! visitors to extract individual fields or elements of structs, maps, arrays, and other
//! nested structures.
//!
//! `tracing`'s support for `valuable` is currently feature flagged.
//!
//! Therefore, when `valuable` support is not enabled also in the subscriber,
//! its implementation falls back to using `fmt::Debug` to record fields that
//! implement `valuable::Valuable`.
use tracing::{info, info_span};
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

    // Record `user` using its `valuable::Valuable` implementation:
    let span = info_span!("Processing", user = user.as_value());

    let _handle = span.enter();
    info!("Nothing to do");
}
