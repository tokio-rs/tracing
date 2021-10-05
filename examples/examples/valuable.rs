//! This example shows how a value may be recorded using valuable and as an alternative recording
//! it without valuable but just using the debug printout. Valuable is more flexible however, as
//! a user can implement a custom `tracing_core::Visit` and implement
//! `tracing_core::Visit::record_value` to handle the valuable and extract the fields, whereas with
//! the debug printout you'd need to parse the string to extract the values.
use tracing::{info_span, info};
#[cfg(tracing_unstable)]
use valuable::Valuable;
#[cfg(tracing_unstable)]
use tracing::field::valuable;

#[cfg_attr(tracing_unstable, derive(Valuable))]
#[derive(Clone, Debug)]
struct User {
    name: String,
    age: u32,
    address: Address, 
}

#[cfg_attr(tracing_unstable, derive(Valuable))]
#[derive(Clone, Debug)]
struct Address {
    country: String,
    city: String,
    street: String,
}


#[cfg(tracing_unstable)]
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
        }
    };

    let span = info_span!("Processing", user=valuable(&user)); 
    let _handle = span.enter();
    info!("Nothing to do");
}


#[cfg(not(tracing_unstable))]
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
        }
    };

    let span = info_span!("Processing", user = ?user); 
    let _handle = span.enter();
    info!("Nothing to do");
}
