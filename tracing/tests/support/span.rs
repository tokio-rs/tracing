#![allow(missing_docs)]
use super::{field, metadata, Parent};
use std::fmt;

/// A mock span.
///
/// This is intended for use with the mock subscriber API in the
/// `subscriber` module.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MockSpan {
    pub(in crate::support) metadata: metadata::Expect,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct NewSpan {
    pub(in crate::support) span: MockSpan,
    pub(in crate::support) fields: field::Expect,
    pub(in crate::support) parent: Option<Parent>,
}

pub fn mock() -> MockSpan {
    MockSpan {
        ..Default::default()
    }
}

impl MockSpan {
    pub fn named<I>(self, name: I) -> Self
    where
        I: Into<String>,
    {
        Self {
            metadata: metadata::Expect {
                name: Some(name.into()),
                ..self.metadata
            },
        }
    }

    pub fn at_level(self, level: tracing::Level) -> Self {
        Self {
            metadata: metadata::Expect {
                level: Some(level),
                ..self.metadata
            },
        }
    }

    pub fn with_target<I>(self, target: I) -> Self
    where
        I: Into<String>,
    {
        Self {
            metadata: metadata::Expect {
                target: Some(target.into()),
                ..self.metadata
            },
        }
    }

    pub fn with_explicit_parent(self, parent: Option<&str>) -> NewSpan {
        let parent = match parent {
            Some(name) => Parent::Explicit(name.into()),
            None => Parent::ExplicitRoot,
        };
        NewSpan {
            parent: Some(parent),
            span: self,
            ..Default::default()
        }
    }

    pub fn with_contextual_parent(self, parent: Option<&str>) -> NewSpan {
        let parent = match parent {
            Some(name) => Parent::Contextual(name.into()),
            None => Parent::ContextualRoot,
        };
        NewSpan {
            parent: Some(parent),
            span: self,
            ..Default::default()
        }
    }

    pub fn name(&self) -> Option<&str> {
        self.metadata.name.as_ref().map(String::as_ref)
    }

    pub fn with_field<I>(self, fields: I) -> NewSpan
    where
        I: Into<field::Expect>,
    {
        NewSpan {
            span: self,
            fields: fields.into(),
            ..Default::default()
        }
    }

    pub(in crate::support) fn check_metadata(&self, actual: &tracing::Metadata<'_>) {
        self.metadata.check(actual, format_args!("span {}", self));
        assert!(actual.is_span(), "expected a span but got {:?}", actual);
    }
}

impl fmt::Display for MockSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.metadata.name.is_some() {
            write!(f, "a span{}", self.metadata)
        } else {
            write!(f, "any span{}", self.metadata)
        }
    }
}

impl From<MockSpan> for NewSpan {
    fn from(span: MockSpan) -> Self {
        Self {
            span,
            ..Default::default()
        }
    }
}

impl NewSpan {
    pub fn with_explicit_parent(self, parent: Option<&str>) -> NewSpan {
        let parent = match parent {
            Some(name) => Parent::Explicit(name.into()),
            None => Parent::ExplicitRoot,
        };
        NewSpan {
            parent: Some(parent),
            ..self
        }
    }

    pub fn with_contextual_parent(self, parent: Option<&str>) -> NewSpan {
        let parent = match parent {
            Some(name) => Parent::Contextual(name.into()),
            None => Parent::ContextualRoot,
        };
        NewSpan {
            parent: Some(parent),
            ..self
        }
    }

    pub fn with_field<I>(self, fields: I) -> NewSpan
    where
        I: Into<field::Expect>,
    {
        NewSpan {
            fields: fields.into(),
            ..self
        }
    }
}

impl fmt::Display for NewSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a new span{}", self.span.metadata)?;
        if !self.fields.is_empty() {
            write!(f, " with {}", self.fields)?;
        }
        Ok(())
    }
}
