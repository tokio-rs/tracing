use super::*;

use log::*;
use std::str::FromStr;
use tracing_subscriber::{
    filter::{filter_fn, Targets},
    prelude::*,
};

#[test]
fn default_debug() -> Result<(), std::io::Error> {
    // Reproduces https://github.com/tokio-rs/tracing/issues/1563

    mod inner {
        use super::*;

        #[tracing::instrument]
        pub fn events(yak: u32) {
            debug!("inner - yak: {} - this is debug", yak);
            info!("inner - yak: {} - this is info", yak);
            warn!("inner - yak: {} - this is warn", yak);
        }
    }

    let filter = Targets::from_str("debug,layer_filters::targets::inner=warn").unwrap();

    let fmt_layer =
        tracing_subscriber::layer::Identity::new().with_filter(filter_fn(move |_meta| true));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    debug!("before inner");

    inner::events(11111);

    debug!("after inner");

    Ok(())
}
