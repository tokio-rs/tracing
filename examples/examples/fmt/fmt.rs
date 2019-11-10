#![deny(rust_2018_idioms)]
use tracing::debug;

mod yak_shave;

fn main() {
    let subscriber = tracing_subscriber::fmt::Subscriber::new();

    tracing::subscriber::with_default(subscriber, || {
        let number_of_yaks = 3;
        debug!("preparing to shave {} yaks", number_of_yaks);

        let number_shaved = yak_shave::shave_all(number_of_yaks);

        debug!(
            message = "yak shaving completed.",
            all_yaks_shaved = number_shaved == number_of_yaks,
        );
    });
}
