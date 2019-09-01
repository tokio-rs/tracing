#![deny(rust_2018_idioms)]
mod yak_shave;

fn main() {
    use tracing_subscriber::{fmt, Filter};

    let subscriber = fmt::FmtSubscriber::builder()
        .with_filter(Filter::from_default_env())
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let number_of_yaks = 3;
        tracing::debug!("preparing to shave {} yaks", number_of_yaks);

        let number_shaved = yak_shave::shave_all(number_of_yaks);

        tracing::debug!(
            message = "yak shaving completed.",
            all_yaks_shaved = number_shaved == number_of_yaks,
        );
    });
}
