use std::fmt;
use tracing_core::Metadata;

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub(crate) struct ExpectedMetadata {
    pub(crate) name: Option<String>,
    pub(crate) level: Option<tracing::Level>,
    pub(crate) target: Option<String>,
}

impl ExpectedMetadata {
    /// Checks the given metadata against this expected metadata and panics if
    /// there is a mismatch.
    ///
    /// The context `ctx` should fit into the followint sentence:
    ///
    /// > expected {ctx} named `expected_name`, but got one named `actual_name`
    ///
    /// Examples could be:
    /// * a new span
    /// * to enter a span
    /// * an event
    ///
    /// # Panics
    ///
    /// This method will panic if any of the expectations that have been
    /// specified are noto met.
    ///
    pub(crate) fn check(
        &self,
        actual: &Metadata<'_>,
        ctx: impl fmt::Display,
        collector_name: &str,
    ) {
        if let Some(ref expected_name) = self.name {
            let actual_name = actual.name();
            assert!(
                expected_name == actual_name,
                "{}",
                format_args!(
                    "\n[{collector_name}] expected {ctx} named `{expected_name}`,\n\
                    [{collector_name}] but got one named `{actual_name}` instead."
                ),
            )
        }

        if let Some(ref expected_level) = self.level {
            let actual_level = actual.level();
            assert!(
                expected_level == actual_level,
                "{}",
                format_args!(
                    "\n[{collector_name}] expected {ctx} at level `{expected_level:?}`,\n\
                    [{collector_name}] but got one at level `{actual_level:?}` instead."
                ),
            )
        }

        if let Some(ref expected_target) = self.target {
            let actual_target = actual.target();
            assert!(
                expected_target == actual_target,
                "{}",
                format_args!(
                    "\n[{collector_name}] expected {ctx} with target `{expected_target}`,\n\
                    [{collector_name}] but got one with target `{actual_target}` instead."
                ),
            )
        }
    }

    pub(crate) fn has_expectations(&self) -> bool {
        self.name.is_some() || self.level.is_some() || self.target.is_some()
    }
}

impl fmt::Display for ExpectedMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref name) = self.name {
            write!(f, " named `{}`", name)?;
        }

        if let Some(ref level) = self.level {
            write!(f, " at the `{:?}` level", level)?;
        }

        if let Some(ref target) = self.target {
            write!(f, " with target `{}`", target)?;
        }

        Ok(())
    }
}
