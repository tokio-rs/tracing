#![allow(unreachable_code)]

#[tracing::instrument(level = "trace", fields(() = "test"))]
fn test_fn() -> &'static str {}

fn main() {}
