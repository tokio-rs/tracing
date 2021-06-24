use tracing_core::{
    collect::{Collect, Interest},
    Metadata,
};

#[allow(unreachable_pub)] // https://github.com/rust-lang/rust/issues/57411
pub use tracing_core::metadata::{LevelFilter, ParseLevelFilterError as ParseError};

// === impl LevelFilter ===

impl<C: Collect> crate::Subscribe<C> for LevelFilter {
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if self >= metadata.level() {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn enabled(&self, metadata: &Metadata<'_>, _: crate::subscribe::Context<'_, C>) -> bool {
        self >= metadata.level()
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        (*self).into()
    }
}
