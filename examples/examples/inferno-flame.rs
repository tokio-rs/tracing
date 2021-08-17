use std::{
    env,
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};
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

fn make_flamegraph(tmpdir: &Path, out: &Path) {
    println!("outputting flamegraph to {}", out.display());
    let inf = File::open(tmpdir.join(PATH)).unwrap();
    let reader = BufReader::new(inf);

    let out = File::create(out).unwrap();
    let writer = BufWriter::new(out);

    let mut opts = inferno::flamegraph::Options::default();
    inferno::flamegraph::from_reader(&mut opts, reader, writer).unwrap();
}

fn main() {
    let out = if let Some(arg) = env::args().nth(1) {
        PathBuf::from(arg)
    } else {
        let mut path = env::current_dir().expect("failed to read current directory");
        path.push("tracing-flame-inferno.svg");
        path
    };

    // setup the flame layer
    let tmp_dir = tempfile::Builder::new()
        .prefix("flamegraphs")
        .tempdir()
        .expect("failed to create temporary directory");
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
    make_flamegraph(tmp_dir.path(), out.as_ref());
}
