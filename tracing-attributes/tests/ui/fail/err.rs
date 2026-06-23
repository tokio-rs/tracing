#![allow(unused)]

use std::convert::TryFrom;

use tracing::instrument;

#[instrument(err(Display, |x| x,,,,,))]
fn err_commas() -> Result<u8, std::num::TryFromIntError> {
    u8::try_from(1234)
}

#[instrument(ret(,,,,,Display, |x| x))]
fn err_commas_2() -> Result<u8, std::num::TryFromIntError> {
    u8::try_from(1234)
}

fn main() {}
