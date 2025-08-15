//! An example of using the [`record`][crate::record] macro to add a field to a span.
#![deny(rust_2018_idioms)]
#![allow(clippy::try_err)]

use rand::{seq::SliceRandom, thread_rng};
use tracing::{instrument, record};

fn main() {
    tracing_subscriber::fmt()
        // enable everything
        .with_max_level(tracing::Level::TRACE)
        // sets this to be the default, global collector for this application.
        .init();

    do_something();
}

#[instrument(fields(important_number = tracing::field::Empty))]
fn do_something() {
    let important_number = get_important_number();
    record!("important_number", %important_number);

    tracing::info!("log entry with important number");
}

fn get_important_number() -> u8 {
    let numbers = [42, 13];
    *numbers.choose(&mut thread_rng()).unwrap()
}
