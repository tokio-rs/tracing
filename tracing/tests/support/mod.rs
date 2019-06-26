#![allow(dead_code)]
pub mod event;
pub mod field;
mod metadata;
pub mod span;
pub mod subscriber;

extern crate tracing_core;

#[derive(Debug, Eq, PartialEq)]
pub(in support) enum Parent {
    ContextualRoot,
    Contextual(String),
    ExplicitRoot,
    Explicit(String),
}
