use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;
use tempdir::TempDir;
use tracing::{span, Level};
use tracing_flame::FlameLayer;
use tracing_subscriber::{prelude::*, registry::Registry};

static PATH: &str = "flame.folded";

fn setup_global_subscriber(dir: &Path) -> impl Drop {
    let (flame_layer, _guard) = FlameLayer::with_file(dir.join(PATH)).unwrap();

    let subscriber = Registry::default().with(flame_layer);

    tracing::subscriber::set_global_default(subscriber).unwrap();

    _guard
}

fn make_flamegraph(dir: &Path) {
    let inf = File::open(dir.join(PATH)).unwrap();
    let reader = BufReader::new(inf);

    let out = File::create(dir.join("inferno.svg")).unwrap();
    let writer = BufWriter::new(out);

    let mut opts = inferno::flamegraph::Options::default();
    inferno::flamegraph::from_reader(&mut opts, reader, writer).unwrap();
}

fn main() {
    // setup the flame layer
    let tmp_dir = TempDir::new("flamegraphs").unwrap();
    let guard = setup_global_subscriber(tmp_dir.path());

    // do a bunch of span entering and exiting to simulate a program running
    span!(Level::ERROR, "outer").in_scope(|| {
        sleep(Duration::from_millis(10));
        span!(Level::ERROR, "Inner").in_scope(|| {
            sleep(Duration::from_millis(50));
            span!(Level::ERROR, "Innermost").in_scope(|| {
                sleep(Duration::from_millis(50));
            });
        });
        sleep(Duration::from_millis(5));
    });
    sleep(Duration::from_millis(500));

    // drop the guard to make sure the layer flushes its output then read the
    // output to create the flamegraph
    drop(guard);
    make_flamegraph(tmp_dir.path());
}
