use std::thread::sleep;
use std::time::Duration;
use tempdir::TempDir;
use tracing::{span, Level};
use tracing_flame::FlameSubscriber;
use tracing_subscriber::{prelude::*, registry::Registry};

#[test]
fn capture_supported() {
    {
        let tmp_dir = TempDir::new("flamegraphs").unwrap();
        let (flame_layer, _guard) =
            FlameSubscriber::with_file(tmp_dir.path().join("tracing.folded")).unwrap();

        let subscriber = Registry::default().with(flame_layer);

        tracing::collect::set_global_default(subscriber).expect("Could not set global default");

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
