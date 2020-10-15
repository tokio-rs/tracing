use tracing_core::{
    collector::{Collect, Interest},
    Metadata,
};

#[allow(unreachable_pub)] // https://github.com/rust-lang/rust/issues/57411
pub use tracing_core::metadata::{LevelFilter, ParseLevelFilterError as ParseError};

// === impl LevelFilter ===

impl<S: Collect> crate::Subscribe<S> for LevelFilter {
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if self >= metadata.level() {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn enabled(&self, metadata: &Metadata<'_>, _: crate::subscribe::Context<'_, S>) -> bool {
        self >= metadata.level()
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        self.clone().into()
    }
}
