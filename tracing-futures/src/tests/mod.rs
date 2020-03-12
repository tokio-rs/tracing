#[cfg_attr(
    not(feature = "std-future"),
    path = "../../../tracing/tests/support/mod.rs"
)]
// We don't need all the test support code that other crates need, so don't
// warn us if it's unused.
#[allow(dead_code, unreachable_pub)]
mod support;

// Tests for `futures` 0.1.
#[cfg(feature = "futures-01")]
mod futures_01;

// Tests for `std::future that require only `std::future` (and not the `futures`
// crate).
#[cfg(feature = "std-future")]
mod std_future;

// Tests that require both `std::future` *and* the `futures` crate (e.g.
// streams, sinks, etc).
#[cfg(feature = "futures-03")]
mod futures_03;
