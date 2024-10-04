//! Define the ancestry of an event or span.
//!
//! See the documentation on the [`Ancestry`] enum for further details.

use tracing_core::{
    span::{self, Attributes},
    Event,
};

use crate::span::{ActualSpan, ExpectedSpan};

/// The ancestry of an event or span.
///
/// An event or span can have an explicitly assigned parent, or be an explicit root. Otherwise,
/// an event or span may have a contextually assigned parent or in the final case will be a
/// contextual root.
#[derive(Debug, Eq, PartialEq)]
pub enum ExpectedAncestry {
    /// The event or span has an explicitly assigned parent (created with `parent: span_id`) span.
    HasExplicitParent(ExpectedSpan),
    /// The event or span is an explicitly defined root. It was created with `parent: None` and
    /// has no parent.
    IsExplicitRoot,
    /// The event or span has a contextually assigned parent span. It has no explicitly assigned
    /// parent span, nor has it been explicitly defined as a root (it was created without the
    /// `parent:` directive). There was a span in context when this event or span was created.
    HasContextualParent(ExpectedSpan),
    /// The event or span is a contextual root. It has no explicitly assigned parent, nor has it
    /// been explicitly defined as a root (it was created without the `parent:` directive).
    /// Additionally, no span was in context when this event or span was created.
    IsContextualRoot,
}

pub(crate) enum ActualAncestry {
    HasExplicitParent(ActualSpan),
    IsExplicitRoot,
    HasContextualParent(ActualSpan),
    IsContextualRoot,
}

impl ExpectedAncestry {
    #[track_caller]
    pub(crate) fn check(
        &self,
        actual_ancestry: &ActualAncestry,
        ctx: impl std::fmt::Display,
        collector_name: &str,
    ) {
        match (self, actual_ancestry) {
            (Self::IsExplicitRoot, ActualAncestry::IsExplicitRoot) => {}
            (Self::IsContextualRoot, ActualAncestry::IsContextualRoot) => {}
            (
                Self::HasExplicitParent(expected_parent),
                ActualAncestry::HasExplicitParent(actual_parent),
            ) => {
                expected_parent.check(
                    actual_parent,
                    format_args!("{ctx} to have an explicit parent span"),
                    collector_name,
                );
            }
            (
                Self::HasContextualParent(expected_parent),
                ActualAncestry::HasContextualParent(actual_parent),
            ) => {
                println!("----> [{collector_name}] check {expected_parent:?} against actual parent with Id={id:?}", id = actual_parent.id());
                expected_parent.check(
                    actual_parent,
                    format_args!("{ctx} to have a contextual parent span"),
                    collector_name,
                );
            }
            _ => {
                // Ancestry types don't match at all.
                let expected_description = match self {
                    Self::IsExplicitRoot => "be an explicit root",
                    Self::HasExplicitParent(_) => "have an explicit parent span",
                    Self::IsContextualRoot => "be a contextual root",
                    Self::HasContextualParent(_) => "have a contextual parent span",
                };

                let actual_description = match actual_ancestry {
                    ActualAncestry::IsExplicitRoot => "is actually an explicit root",
                    ActualAncestry::HasExplicitParent(_) => "actually has an explicit parent span",
                    ActualAncestry::IsContextualRoot => "is actually a contextual root",
                    ActualAncestry::HasContextualParent(_) => {
                        "actually has a contextual parent span"
                    }
                };

                panic!(
                    "{}",
                    format!(
                        "[{collector_name}] expected {ctx} to {expected_description}, \
                        but it {actual_description}"
                    )
                );
            }
        }
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
    actual_span: impl FnOnce(&span::Id) -> Option<ActualSpan>,
) -> ActualAncestry {
    if item.is_contextual() {
        if let Some(parent_id) = lookup_current() {
            let contextual_parent_span = actual_span(&parent_id).expect(
                "tracing-mock: contextual parent cannot \
                            be looked up by ID. Was it recorded correctly?",
            );
            ActualAncestry::HasContextualParent(contextual_parent_span)
        } else {
            ActualAncestry::IsContextualRoot
        }
    } else if item.is_root() {
        ActualAncestry::IsExplicitRoot
    } else {
        let parent_id = item.parent().expect(
            "tracing-mock: is_contextual=false is_root=false \
                        but no explicit parent found. This is a bug!",
        );
        let explicit_parent_span = actual_span(parent_id).expect(
            "tracing-mock: explicit parent cannot be looked \
                        up by ID. Is the provided Span ID valid: {parent_id}",
        );
        ActualAncestry::HasExplicitParent(explicit_parent_span)
    }
}
