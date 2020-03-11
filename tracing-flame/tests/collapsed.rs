use std::thread::sleep;
use std::time::Duration;
use tracing::{span, Level};
use tracing_flame::FlameLayer;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::{fmt, prelude::*, registry::Registry};

#[test]
fn capture_supported() {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    {
        let subscriber = fmt::Layer::builder()
            .with_target(false)
            .finish()
            .and_then(FlameLayer::write_to_file("./tracing.folded").unwrap())
            .and_then(filter)
            .with_subscriber(Registry::default());

        tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");

        {
            let span = span!(Level::ERROR, "outer");
            let _guard = span.enter();
            sleep(Duration::from_millis(10));

            {
                let span = span!(Level::ERROR, "Inner");
                let _guard = span.enter();
                sleep(Duration::from_millis(50));

                {
                    let span = span!(Level::ERROR, "Innermost");
                    let _guard = span.enter();
                    sleep(Duration::from_millis(50));
                }
            }

            sleep(Duration::from_millis(5));
        }

        sleep(Duration::from_millis(500));
    }
}
