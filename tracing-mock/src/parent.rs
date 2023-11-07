/// The parent of an event or span.
///
/// This enum is used to represent the expected and the actual parent of an
/// event or a span.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Parent {
    /// The event or span is contextually a root - it has no parent.
    ContextualRoot,
    /// The event or span has a contextually assigned parent, with the specified name.
    Contextual(String),
    /// The event or span is explicitly a root, it was created with `parent: None`.
    ExplicitRoot,
    /// The event or span has an explicit parent with the specified name, it was created with
    /// `parent: span_id`.
    Explicit(String),
}

impl Parent {
    #[track_caller]
    pub(crate) fn check(
        &self,
        actual_parent: &Parent,
        ctx: impl std::fmt::Display,
        collector_name: &str,
    ) {
        let expected_description = |parent: &Parent| match parent {
            Self::ExplicitRoot => "be an explicit root".to_string(),
            Self::Explicit(name) => format!("have an explicit parent with name='{name}'"),
            Self::ContextualRoot => "be a contextual root".to_string(),
            Self::Contextual(name) => format!("have a contextual parent with name='{name}'"),
        };

        let actual_description = |parent: &Parent| match parent {
            Self::ExplicitRoot => "was actually an explicit root".to_string(),
            Self::Explicit(name) => format!("actually has an explicit parent with name='{name}'"),
            Self::ContextualRoot => "was actually a contextual root".to_string(),
            Self::Contextual(name) => {
                format!("actually has a contextual parent with name='{name}'")
            }
        };

        assert_eq!(
            self,
            actual_parent,
            "[{collector_name}] expected {ctx} to {expected_description}, but {actual_description}",
            expected_description = expected_description(self),
            actual_description = actual_description(actual_parent)
        );
    }
}
