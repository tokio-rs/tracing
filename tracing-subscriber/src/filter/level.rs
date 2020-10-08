use tracing_core::{
    collector::{Collector, Interest},
    Metadata,
};

#[allow(unreachable_pub)] // https://github.com/rust-lang/rust/issues/57411
pub use tracing_core::metadata::{LevelFilter, ParseLevelFilterError as ParseError};

// === impl LevelFilter ===

impl<S: Collector> crate::Subscriber<S> for LevelFilter {
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if self >= metadata.level() {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn enabled(&self, metadata: &Metadata<'_>, _: crate::subscriber::Context<'_, S>) -> bool {
        self >= metadata.level()
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        self.clone().into()
    }
}
