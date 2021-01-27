use std::thread::sleep;
use std::time::Duration;
use tempdir::TempDir;
use tracing::{span, Level};
use tracing_flame::FlameSubscriber;
use tracing_subscriber::{prelude::*, registry::Registry};

#[test]
fn capture_supported() {
    let tmp_dir = TempDir::new("flamegraphs").unwrap();
    let path = tmp_dir.path().join("tracing.folded");
    let (flame_layer, flame_guard) = FlameSubscriber::with_file(&path).unwrap();

    let subscriber = Registry::default().with(flame_layer);

    tracing::collect::set_global_default(subscriber).expect("Could not set global default");
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
    flame_guard.flush().unwrap();

    let traces = std::fs::read_to_string(&path).unwrap();
    println!("{}", traces);
    assert_eq!(5, traces.lines().count());
}
