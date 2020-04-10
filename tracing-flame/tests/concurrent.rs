use std::thread::sleep;
use std::time::Duration;
use tempdir::TempDir;
use tracing::{span, Level};
use tracing_flame::FlameLayer;
use tracing_subscriber::{prelude::*, registry::Registry};

#[test]
fn capture_supported() {
    let tmp_dir = TempDir::new("flamegraphs").unwrap();
    let (flame_layer, _guard) =
        FlameLayer::with_file(tmp_dir.path().join("tracing.folded")).unwrap();

    let subscriber = Registry::default().with(flame_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
    let span = span!(Level::ERROR, "main");
    let _guard = span.enter();

    let thread = span!(Level::ERROR, "outer").in_scope(|| {
        sleep(Duration::from_millis(10));
        let span = span!(Level::ERROR, "Inner");
        let thread = std::thread::spawn(move || {
            span.in_scope(|| {
                sleep(Duration::from_millis(50));
            });
        });
        sleep(Duration::from_millis(20));
        thread
    });

    sleep(Duration::from_millis(100));

    thread.join().unwrap();
}
