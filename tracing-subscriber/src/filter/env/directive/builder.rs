use core::fmt::Debug;

use tracing_core::LevelFilter;

use crate::filter::{env::field, ValueMatch};

use super::Directive;

/// A [builder] for constructing new [`Directive`]s.
///
/// [builder]: https://rust-unofficial.github.io/patterns/patterns/creational/builder.html
#[derive(Debug, Clone)]
#[must_use]
pub struct Builder {
    in_span: Option<String>,
    fields: Vec<field::Match>,
    target: Option<String>,
    level: LevelFilter,
}

// ==== impl Builder ====

impl Builder {
    /// Sets the [`LevelFilter`] which is applied to any [span] matching the directive.
    ///
    /// The matching algorithm is explained in the [`Directive`]s documentation.
    ///
    /// [span]: mod@tracing::span
    pub fn with_level(self, level: impl Into<LevelFilter>) -> Self {
        Self {
            level: level.into(),
            ..self
        }
    }

    /// Sets the [target] prefix used for matching directives to spans.
    ///
    /// The matching algorithm is explained in the [`Directive`]s documentation.
    ///
    /// [target]: fn@tracing::Metadata::target
    pub fn with_target_prefix(self, prefix: impl Into<String>) -> Self {
        Self {
            target: Some(prefix.into()).filter(|target| !target.is_empty()),
            ..self
        }
    }

    /// Sets the [span] used for matching directives to spans.
    ///
    /// The matching algorithm is explained in the [`Directive`]s documentation.
    ///
    /// [span]: mod@tracing::span
    pub fn with_span_name(self, name: impl Into<String>) -> Self {
        Self {
            in_span: Some(name.into()),
            ..self
        }
    }

    /// Adds a [field] used for matching directives to spans.
    ///
    /// Optionally a [value] can be provided, too.
    ///
    /// The matching algorithm is explained in the [`Directive`]s documentation.
    ///
    /// [field]: fn@tracing::Metadata::fields
    /// [value]: tracing#recording-fields
    pub fn with_field(mut self, name: impl Into<String>, value: Option<ValueMatch>) -> Self {
        self.fields.push(field::Match {
            name: name.into(),
            value: value.map(field::ValueMatch::from),
        });
        self
    }

    /// Builds a new [`Directive`].
    pub fn build(self) -> Directive {
        let Self {
            in_span,
            fields,
            target,
            level,
        } = self;
        Directive {
            in_span,
            fields,
            target,
            level,
        }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            in_span: None,
            fields: Vec::new(),
            target: None,
            level: LevelFilter::TRACE,
        }
    }
}
