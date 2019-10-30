#![deny(rust_2018_idioms)]
use std::io;
use tracing::{debug, Level};
use tracing_subscriber::{
    layer::Layer,
    registry::{FmtLayer, Registry},
};

mod yak_shave;

fn main() {
    let subscriber = tracing_subscriber::fmt::Subscriber::new();
    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");

    let number_of_yaks = 3;
    debug!("preparing to shave {} yaks", number_of_yaks);

    let number_shaved = yak_shave::shave_all(number_of_yaks);

    debug!(
        message = "yak shaving completed.",
        all_yaks_shaved = number_shaved == number_of_yaks,
    );
}
