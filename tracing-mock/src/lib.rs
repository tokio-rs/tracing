#![doc = include_str!("../README.md")]
pub mod collector;
pub mod event;
pub mod expect;
pub mod field;
mod metadata;
pub mod span;

#[cfg(feature = "tracing-subscriber")]
pub mod subscriber;

#[derive(Debug, Eq, PartialEq)]
pub enum Parent {
    ContextualRoot,
    Contextual(String),
    ExplicitRoot,
    Explicit(String),
}

impl Parent {
    pub fn check_parent_name(
        &self,
        parent_name: Option<&str>,
        provided_parent: Option<tracing_core::span::Id>,
        ctx: impl std::fmt::Display,
        collector_name: &str,
    ) {
        match self {
            Parent::ExplicitRoot => {
                assert!(
                    provided_parent.is_none(),
                    "[{}] expected {} to be an explicit root, but its parent was actually {:?} (name: {:?})",
                    collector_name,
                    ctx,
                    provided_parent,
                    parent_name,
                );
            }
            Parent::Explicit(expected_parent) => {
                assert!(
                    provided_parent.is_some(),
                    "[{}] expected {} to have explicit parent {}, but it has no explicit parent",
                    collector_name,
                    ctx,
                    expected_parent,
                );
                assert_eq!(
                    Some(expected_parent.as_ref()),
                    parent_name,
                    "[{}] expected {} to have explicit parent {}, but its parent was actually {:?} (name: {:?})",
                    collector_name,
                    ctx,
                    expected_parent,
                    provided_parent,
                    parent_name,
                );
            }
            Parent::ContextualRoot => {
                assert!(
                    provided_parent.is_none(),
                    "[{}] expected {} to be a contextual root, but its parent was actually {:?} (name: {:?})",
                    collector_name,
                    ctx,
                    provided_parent,
                    parent_name,
                );
                assert!(
                    parent_name.is_none(),
                    "[{}] expected {} to be contextual a root, but we were inside span {:?}",
                    collector_name,
                    ctx,
                    parent_name,
                );
            }
            Parent::Contextual(expected_parent) => {
                assert!(provided_parent.is_none(),
                    "[{}] expected {} to have a contextual parent\nbut it has the explicit parent {:?} (name: {:?})",
                    collector_name,
                    ctx,
                    provided_parent,
                    parent_name,
                );
                assert_eq!(
                    Some(expected_parent.as_ref()),
                    parent_name,
                    "[{}] expected {} to have contextual parent {:?}, but got {:?}",
                    collector_name,
                    ctx,
                    expected_parent,
                    parent_name,
                );
            }
        }
    }
}
