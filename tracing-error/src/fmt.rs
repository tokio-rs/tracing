use std::fmt;
pub use tracing_subscriber::fmt::format::{DefaultFields, FormatFields};

pub struct Backtrace<'a, T> {
    inner: &'a T,
}
