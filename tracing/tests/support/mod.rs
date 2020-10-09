#![allow(dead_code)]
pub mod collector;
pub mod event;
pub mod field;
mod metadata;
pub mod span;

#[derive(Debug, Eq, PartialEq)]
pub(in crate::support) enum Parent {
    ContextualRoot,
    Contextual(String),
    ExplicitRoot,
    Explicit(String),
}
