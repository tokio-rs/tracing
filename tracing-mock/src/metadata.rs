use std::fmt;
use tracing_core::Metadata;

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub(crate) struct ExpectedMetadata {
    pub(crate) name: Option<String>,
    pub(crate) level: Option<tracing::Level>,
    pub(crate) target: Option<String>,
}

impl ExpectedMetadata {
    pub(crate) fn check(
        &self,
        actual: &Metadata<'_>,
        ctx: fmt::Arguments<'_>,
        collector_name: &str,
    ) {
        if let Some(ref expected_name) = self.name {
            let name = actual.name();
            assert!(
                expected_name == name,
                "\n[{}] expected {} to be named `{}`, but got one named `{}`",
                collector_name,
                ctx,
                expected_name,
                name
            )
        }

        if let Some(ref expected_level) = self.level {
            let level = actual.level();
            assert!(
                expected_level == level,
                "\n[{}] expected {} to be at level `{:?}`, but it was at level `{:?}` instead",
                collector_name,
                ctx,
                expected_level,
                level,
            )
        }

        if let Some(ref expected_target) = self.target {
            let target = actual.target();
            assert!(
                expected_target == target,
                "\n[{}] expected {} to have target `{}`, but it had target `{}` instead",
                collector_name,
                ctx,
                expected_target,
                target,
            )
        }
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
