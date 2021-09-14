use super::*;
use tracing_subscriber::{
    filter::{filter_fn, Targets},
    prelude::*,
};

#[test]
#[cfg_attr(not(feature = "tracing-log"), ignore)]
fn log_events() {
    // Reproduces https://github.com/tokio-rs/tracing/issues/1563
    mod inner {
        pub(super) const MODULE_PATH: &str = module_path!();

        #[tracing::instrument]
        pub(super) fn logs() {
            log::debug!("inner");
        }
    }

    let filter = Targets::new()
        .with_default(LevelFilter::DEBUG)
        .with_target(inner::MODULE_PATH, LevelFilter::WARN);

    let layer =
        tracing_subscriber::layer::Identity::new().with_filter(filter_fn(move |_meta| true));

    let _guard = tracing_subscriber::registry()
        .with(filter)
        .with(layer)
        .set_default();

    inner::logs();
}
