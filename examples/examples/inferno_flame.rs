use eyre::{ErrReport, WrapErr};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::thread::sleep;
use std::time::Duration;
use tracing::{span, Level};
use tracing_flame::{FlameGuard, FlameLayer};
use tracing_subscriber::{prelude::*, registry::Registry};

static PATH: &str = "./flame.folded";

fn setup_global_subscriber() -> Result<FlameGuard<BufWriter<File>>, ErrReport> {
    let flame_layer =
        FlameLayer::write_to_file(PATH).wrap_err("Unable to instantiate tracing FlameLayer")?;
    let _guard = flame_layer.flush_on_drop();

    let subscriber = Registry::default().with(flame_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(_guard)
}

fn make_flamegraph() -> Result<(), ErrReport> {
    let inf = File::open(PATH)
        .wrap_err_with(|| format!("Failed open folded stack trace file. path={}", PATH))?;
    let reader = BufReader::new(inf);

    let out =
        File::create("./inferno.svg").wrap_err("Failed to create file for flamegraph image")?;
    let writer = BufWriter::new(out);

    let mut opts = inferno::flamegraph::Options::default();
    inferno::flamegraph::from_reader(&mut opts, reader, writer)?;

    Ok(())
}

fn main() -> Result<(), ErrReport> {
    {
        let _guard = setup_global_subscriber().wrap_err("Unable to setup global subscriber");

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

    make_flamegraph().wrap_err("Unable to create flamegraph svg")?;

    Ok(())
}
