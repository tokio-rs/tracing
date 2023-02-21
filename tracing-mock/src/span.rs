#![allow(missing_docs)]
use super::{expect, field::ExpectedFields, metadata::ExpectedMetadata, Parent};
use std::fmt;

/// A mock span.
///
/// This is intended for use with the mock subscriber API in the
/// `subscriber` module.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct ExpectedSpan {
    pub(crate) metadata: ExpectedMetadata,
}

#[derive(Default, Eq, PartialEq)]
pub struct NewSpan {
    pub(crate) span: ExpectedSpan,
    pub(crate) fields: ExpectedFields,
    pub(crate) parent: Option<Parent>,
}

pub fn named<I>(name: I) -> ExpectedSpan
where
    I: Into<String>,
{
    expect::span().named(name)
}

impl ExpectedSpan {
    pub fn named<I>(self, name: I) -> Self
    where
        I: Into<String>,
    {
        Self {
            metadata: ExpectedMetadata {
                name: Some(name.into()),
                ..self.metadata
            },
        }
    }

    pub fn at_level(self, level: tracing::Level) -> Self {
        Self {
            metadata: ExpectedMetadata {
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
            metadata: ExpectedMetadata {
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

    pub fn level(&self) -> Option<tracing::Level> {
        self.metadata.level
    }

    pub fn target(&self) -> Option<&str> {
        self.metadata.target.as_deref()
    }

    pub fn with_field<I>(self, fields: I) -> NewSpan
    where
        I: Into<ExpectedFields>,
    {
        NewSpan {
            span: self,
            fields: fields.into(),
            ..Default::default()
        }
    }
}

impl fmt::Debug for ExpectedSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("MockSpan");

        if let Some(name) = self.name() {
            s.field("name", &name);
        }

        if let Some(level) = self.level() {
            s.field("level", &format_args!("{:?}", level));
        }

        if let Some(target) = self.target() {
            s.field("target", &target);
        }

        s.finish()
    }
}

impl fmt::Display for ExpectedSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.metadata.name.is_some() {
            write!(f, "a span{}", self.metadata)
        } else {
            write!(f, "any span{}", self.metadata)
        }
    }
}

impl From<ExpectedSpan> for NewSpan {
    fn from(span: ExpectedSpan) -> Self {
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
        I: Into<ExpectedFields>,
    {
        NewSpan {
            fields: fields.into(),
            ..self
        }
    }

    pub fn check(
        &mut self,
        span: &tracing_core::span::Attributes<'_>,
        get_parent_name: impl FnOnce() -> Option<String>,
        collector_name: &str,
    ) {
        let meta = span.metadata();
        let name = meta.name();
        self.span
            .metadata
            .check(meta, format_args!("span `{}`", name), collector_name);
        let mut checker = self.fields.checker(name, collector_name);
        span.record(&mut checker);
        checker.finish();

        if let Some(expected_parent) = self.parent.as_ref() {
            let actual_parent = get_parent_name();
            expected_parent.check_parent_name(
                actual_parent.as_deref(),
                span.parent().cloned(),
                format_args!("span `{}`", name),
                collector_name,
            )
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

impl fmt::Debug for NewSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("NewSpan");

        if let Some(name) = self.span.name() {
            s.field("name", &name);
        }

        if let Some(level) = self.span.level() {
            s.field("level", &format_args!("{:?}", level));
        }

        if let Some(target) = self.span.target() {
            s.field("target", &target);
        }

        if let Some(ref parent) = self.parent {
            s.field("parent", &format_args!("{:?}", parent));
        }

        if !self.fields.is_empty() {
            s.field("fields", &self.fields);
        }

        s.finish()
    }
}
