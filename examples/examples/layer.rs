use std::cmp::Ordering;
use tracing::{debug, info, instrument, warn, Level, Metadata, Subscriber};
use tracing_subscriber::{
    fmt,
    layer::{Context, Filter, Layer},
    registry::Registry,
};

struct LevelFilter {
    level: Level,
    ordering: Ordering,
}

impl<L, S> Filter<L, S> for LevelFilter
where
    L: Layer<S>,
    S: Subscriber,
{
    fn filter(&self, metadata: &Metadata<'_>, _: &Context<'_, S>) -> bool {
        if metadata.level().cmp(&self.level) == self.ordering {
            true
        } else {
            false
        }
    }
}

impl LevelFilter {
    fn new(level: Level, ordering: Ordering) -> Self {
        Self { level, ordering }
    }
}

fn main() {
    // not inclusive, so this behaves as `>` or `<`, not `>=` or `<=`
    let warn = LevelFilter::new(Level::INFO, Ordering::Greater);
    let debug = LevelFilter::new(Level::INFO, Ordering::Less);

    let warn = fmt::Layer::default().with_filter(warn);
    let debug = fmt::Layer::default().with_filter(debug);

    let subscriber = warn.and_then(debug).with_subscriber(Registry::default());
    tracing::subscriber::set_global_default(subscriber).expect("Unable to set global subscriber");
    call_one();
}

#[instrument(level = "TRACE")]
fn call_one() {
    warn!("This is the warning level");
    info!("This should be ignored");
    call_two();
}

#[instrument(level = "TRACE")]
fn call_two() {
    debug!("This is the debug level!");
    info!("This should be ignored");
}
