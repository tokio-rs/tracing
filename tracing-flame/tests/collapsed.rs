use std::thread::sleep;
use std::time::Duration;
use tracing::{span, Level};
use tracing_flame::FlameLayer;
use tracing_subscriber::{prelude::*, registry::Registry};

#[test]
fn capture_supported() {
    {
        let flame_layer = FlameLayer::with_file("./tracing.folded").unwrap();
        let _guard = flame_layer.flush_on_drop();

        let subscriber = Registry::default().with(flame_layer);

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
