//! Define the ancestry of an event or span.
//!
//! See the documentation on the [`Ancestry`] enum for further details.

use tracing_core::{
    span::{self, Attributes},
    Event,
};

/// The ancestry of an event or span.
///
/// An event or span can have an explicitly assigned parent, or be an explicit root. Otherwise,
/// an event or span may have a contextually assigned parent or in the final case will be a
/// contextual root.
#[derive(Debug, Eq, PartialEq)]
pub enum Ancestry {
    /// The event or span has an explicitly assigned parent (created with `parent: span_id`) with
    /// the specified name.
    HasExplicitParent(String),
    /// The event or span is an explicitly defined root. It was created with `parent: None` and
    /// has no parent.
    IsExplicitRoot,
    /// The event or span has a contextually assigned parent with the specified name. It has no
    /// explicitly assigned parent, nor has it been explicitly defined as a root (it was created
    /// without the `parent:` directive). There was a span in context when this event or span was
    /// created.
    HasContextualParent(String),
    /// The event or span is a contextual root. It has no explicitly assigned parent, nor has it
    /// been explicitly defined as a root (it was created without the `parent:` directive).
    /// Additionally, no span was in context when this event or span was created.
    IsContextualRoot,
}

impl Ancestry {
    #[track_caller]
    pub(crate) fn check(
        &self,
        actual_ancestry: &Ancestry,
        ctx: impl std::fmt::Display,
        collector_name: &str,
    ) {
        let expected_description = |ancestry: &Ancestry| match ancestry {
            Self::IsExplicitRoot => "be an explicit root".to_string(),
            Self::HasExplicitParent(name) => format!("have an explicit parent with name='{name}'"),
            Self::IsContextualRoot => "be a contextual root".to_string(),
            Self::HasContextualParent(name) => {
                format!("have a contextual parent with name='{name}'")
            }
        };

        let actual_description = |ancestry: &Ancestry| match ancestry {
            Self::IsExplicitRoot => "was actually an explicit root".to_string(),
            Self::HasExplicitParent(name) => {
                format!("actually has an explicit parent with name='{name}'")
            }
            Self::IsContextualRoot => "was actually a contextual root".to_string(),
            Self::HasContextualParent(name) => {
                format!("actually has a contextual parent with name='{name}'")
            }
        };

        assert_eq!(
            self,
            actual_ancestry,
            "[{collector_name}] expected {ctx} to {expected_description}, but {actual_description}",
            expected_description = expected_description(self),
            actual_description = actual_description(actual_ancestry)
        );
    }
}

pub(crate) trait HasAncestry {
    fn is_contextual(&self) -> bool;

    fn is_root(&self) -> bool;

    fn parent(&self) -> Option<&span::Id>;
}

impl HasAncestry for &Event<'_> {
    fn is_contextual(&self) -> bool {
        (self as &Event<'_>).is_contextual()
    }

    fn is_root(&self) -> bool {
        (self as &Event<'_>).is_root()
    }

    fn parent(&self) -> Option<&span::Id> {
        (self as &Event<'_>).parent()
    }
}

impl HasAncestry for &Attributes<'_> {
    fn is_contextual(&self) -> bool {
        (self as &Attributes<'_>).is_contextual()
    }

    fn is_root(&self) -> bool {
        (self as &Attributes<'_>).is_root()
    }

    fn parent(&self) -> Option<&span::Id> {
        (self as &Attributes<'_>).parent()
    }
}

/// Determines the ancestry of an actual span or event.
///
/// The rules for determining the ancestry are as follows:
///
/// +------------+--------------+-----------------+---------------------+
/// | Contextual | Current Span | Explicit Parent | Ancestry            |
/// +------------+--------------+-----------------+---------------------+
/// | Yes        | Yes          | -               | HasContextualParent |
/// | Yes        | No           | -               | IsContextualRoot    |
/// | No         | -            | Yes             | HasExplicitParent   |
/// | No         | -            | No              | IsExplicitRoot      |
/// +------------+--------------+-----------------+---------------------+
pub(crate) fn get_ancestry(
    item: impl HasAncestry,
    lookup_current: impl FnOnce() -> Option<span::Id>,
    span_name: impl FnOnce(&span::Id) -> Option<&str>,
) -> Ancestry {
    if item.is_contextual() {
        if let Some(parent_id) = lookup_current() {
            let contextual_parent_name = span_name(&parent_id).expect(
                "tracing-mock: contextual parent cannot \
                            be looked up by ID. Was it recorded correctly?",
            );
            Ancestry::HasContextualParent(contextual_parent_name.to_string())
        } else {
            Ancestry::IsContextualRoot
        }
    } else if item.is_root() {
        Ancestry::IsExplicitRoot
    } else {
        let parent_id = item.parent().expect(
            "tracing-mock: is_contextual=false is_root=false \
                        but no explicit parent found. This is a bug!",
        );
        let explicit_parent_name = span_name(parent_id).expect(
            "tracing-mock: explicit parent cannot be looked \
                        up by ID. Is the provided Span ID valid: {parent_id}",
        );
        Ancestry::HasExplicitParent(explicit_parent_name.to_string())
    }
}
