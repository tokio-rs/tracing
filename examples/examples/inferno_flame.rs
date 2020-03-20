use error::{ErrReport, WrapErr};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::thread::sleep;
use std::time::Duration;
use tracing::{span, Level};
use tracing_flame::{FlameLayer};
use tracing_subscriber::{prelude::*, registry::Registry};

static PATH: &str = "./flame.folded";

fn setup_global_subscriber() -> Result<impl Drop, ErrReport> {
    let (flame_layer, _guard) =
        FlameLayer::with_file(PATH).wrap_err("Unable to instantiate tracing FlameLayer")?;

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

mod error {
    pub use eyre::*;

    pub type ErrReport = eyre::ErrReport<TracingContext>;

    use indenter::Indented;
    use std::any::{Any, TypeId};
    use std::error::Error;
    use std::fmt::Write as _;
    use tracing_error::{ExtractSpanTrace, SpanTrace, SpanTraceStatus};

    pub struct TracingContext {
        span_trace: Option<SpanTrace>,
    }

    fn get_deepest_spantrace<'a>(error: &'a (dyn Error + 'static)) -> Option<&'a SpanTrace> {
        Chain::new(error)
            .rev()
            .flat_map(|error| error.span_trace())
            .next()
    }

    impl EyreContext for TracingContext {
        fn default(error: &(dyn std::error::Error + 'static)) -> Self {
            let span_trace = if get_deepest_spantrace(error).is_none() {
                Some(SpanTrace::capture())
            } else {
                None
            };

            Self {
                span_trace,
            }
        }

        fn member_ref(&self, typeid: TypeId) -> Option<&dyn Any> {
            if typeid == TypeId::of::<SpanTrace>() {
                self.span_trace.as_ref().map(|s| s as &dyn Any)
            } else {
                None
            }
        }

        fn display(
            &self,
            error: &(dyn std::error::Error + 'static),
            f: &mut core::fmt::Formatter<'_>,
        ) -> core::fmt::Result {
            write!(f, "{}", error)?;

            if f.alternate() {
                for cause in Chain::new(error).skip(1) {
                    write!(f, ": {}", cause)?;
                }
            }

            Ok(())
        }

        fn debug(
            &self,
            error: &(dyn std::error::Error + 'static),
            f: &mut core::fmt::Formatter<'_>,
        ) -> core::fmt::Result {
            if f.alternate() {
                return core::fmt::Debug::fmt(error, f);
            }

            let errors = Chain::new(error).rev().enumerate();

            for (n, error) in errors {
                writeln!(f)?;
                write!(Indented::numbered(f, n), "{}", error)?;
            }

            let span_trace = self
                .span_trace
                .as_ref()
                .or_else(|| get_deepest_spantrace(error))
                .expect("SpanTrace capture failed");

            match span_trace.status() {
            SpanTraceStatus::CAPTURED => write!(f, "\n\nSpan Trace:\n{}", span_trace)?,
            SpanTraceStatus::UNSUPPORTED => write!(f, "\n\nWarning: SpanTrace capture is Unsupported.\nEnsure that you've setup an error layer and the versions match")?,
            _ => (),
        }

            Ok(())
        }
    }
}
