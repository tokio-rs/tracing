#![allow(dead_code)]
//! This example shows how a value may be recorded using valuable and as an alternative recording
//! it without valuable but just using the debug printout. Valuable is more flexible however, as
//! a user can implement a custom `tracing_core::Visit` and implement
//! `tracing_core::Visit::record_value` to handle the valuable and extract the fields, whereas with
//! the debug printout you'd need to parse the string to extract the values.
#[cfg(tracing_unstable)]
use tracing::field::valuable;
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

    #[cfg(tracing_unstable)]
    let span = info_span!("Processing", user = valuable(&user));

    #[cfg(not(tracing_unstable))]
    let span = info_span!("Processing", user = ?user);

    let _handle = span.enter();
    info!("Nothing to do");
}
