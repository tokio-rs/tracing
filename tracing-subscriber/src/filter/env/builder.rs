//! A builder API for `env` Filters

use super::EnvFilter;
use std::collections::BTreeMap;
use tracing_core::Level;

/// A filter directive for the builder
pub enum Filter {
    /// Inherit the level from the parent
    ///
    /// This variant can't be used as the root filter level and will
    /// cause the builder to panic.
    Inherit,
    /// Provide any valid tracing Level.
    Level(Level),
}

impl From<Level> for Filter {
    fn from(l: Level) -> Self {
        Self::Level(l)
    }
}

impl Default for Filter {
    fn default() -> Self {
        Self::Inherit
    }
}

/// A builder API for [`EnvFilter`]
///
/// [`EnvFilter`]: ../struct.EnvFilter.html
pub struct EnvBuilder {
    filter: Filter,
    children: BTreeMap<String, EnvBuilder>,
}

impl EnvBuilder {
    /// Add a sub-module to the builder scope
    pub fn child<S: Into<String>>(mut self, name: S, b: EnvBuilder) -> Self {
        self.children.insert(name.into(), b);
        self
    }

    /// Evaluate this builder tree to produce a valid [`EnvFilter`]
    ///
    /// This function may panic if the root filter directive has been
    /// set to `Filter::Inherit`.
    ///
    /// [`EnvFilter`]: ../struct.EnvFilter.html
    pub fn process(self) -> EnvFilter {
        todo!()
    }
}

/// Create an [`EnvBuilder`] with a default log level
///
/// [`EnvBuilder`]: ./struct.EnvBuilder.html
pub fn level<F: Into<Filter>>(filter: F) -> EnvBuilder {
    EnvBuilder {
        filter: filter.into(),
        children: Default::default(),
    }
}

#[test]
fn use_the_api() {
    let _b = level(Level::TRACE).child(
        "my_crate",
        level(Filter::Inherit)
            .child("foo", level(Level::WARN))
            .child("bar", level(Level::ERROR)),
    );

    // Will produce: my_crate::foo=warn,my_crate::bar=error,trace
}
