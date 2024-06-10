#![allow(dead_code)]
//! This example shows how a field value may be recorded using the `valuable`
//! crate (https://crates.io/crates/valuable).
//!
//! `valuable` provides a lightweight but flexible way to record structured data, allowing
//! visitors to extract individual fields or elements of structs, maps, arrays, and other
//! nested structures.
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

    // If the `valuable` feature is enabled, record `user` using its
    // `valuable::Valuable` implementation:
    tracing::info!(valuable = true, user = valuable(&user));
}
