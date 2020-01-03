use std::marker::PhantomData;
use tracing::{debug, info, instrument, warn, Level, Metadata, Subscriber};
use tracing_subscriber::{
    fmt,
    layer::{Context, Filter, Layer},
    registry::Registry,
};

struct FilterFn<F, S> {
    f: F,
    _s: PhantomData<S>,
}

impl<L, S, F> Filter<L, S> for FilterFn<F, S>
where
    L: Layer<S>,
    S: Subscriber,
    F: Fn(&Metadata, &Context<'_, S>) -> bool,
{
    fn filter(&self, metadata: &Metadata<'_>, ctx: &Context<'_, S>) -> bool {
        (self.f)(metadata, ctx)
    }
}

impl<F, S> FilterFn<F, S>
where
    S: Subscriber + 'static,
    F: Fn(&Metadata, &Context<'_, S>) -> bool,
{
    fn new(f: F) -> Self {
        Self { f, _s: PhantomData }
    }
}

fn main() {
    let warn = FilterFn::new(|metadata, _| metadata.level() <= &Level::WARN);
    let debug = FilterFn::new(|metadata, _| metadata.level() >= &Level::DEBUG);

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
