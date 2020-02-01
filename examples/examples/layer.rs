use std::io;
use std::marker::PhantomData;
use tracing::{debug, info, instrument, warn, Level, Metadata, Subscriber};
use tracing_subscriber::{
    fmt,
    layer::{Context, Filter, Layer},
    registry::Registry,
};

// struct FilterFn<F, S> {
//     f: F,
//     _s: PhantomData<S>,
// }

// impl<S, F> Filter<S> for FilterFn<F, S>
// where
//     // L: Layer<S>,
//     S: Subscriber,
//     F: Fn(&Metadata, &Context<'_, S>) -> bool,
// {
//     fn filter(&self, metadata: &Metadata<'_>, ctx: &Context<'_, S>) -> bool {
//         (self.f)(metadata, ctx)
//     }
// }

// impl<F, S> FilterFn<F, S>
// where
//     S: Subscriber + 'static,
//     F: Fn(&Metadata, &Context<'_, S>) -> bool,
// {
//     fn new(f: F) -> Self {
//         Self { f, _s: PhantomData }
//     }
// }

fn main() {
    // let err = FilterFn::new(|metadata, _| {
    //     metadata.level() == &Level::WARN || metadata.level() == &Level::ERROR
    // });
    // let info = FilterFn::new(|metadata, _| metadata.level() <= &Level::INFO);

    // let err = fmt::Layer::builder()
    //     .with_writer(io::stderr)
    //     .finish()
    //     .with_filter(err);
    // let info = fmt::Layer::builder()
    //     .with_writer(io::stdout)
    //     .finish()
    //     .with_filter(info);

    // let subscriber = info.and_then(err).with_subscriber(Registry::default());
    // tracing::subscriber::set_global_default(subscriber).expect("Unable to set global subscriber");
    call_one();
}

#[instrument]
fn call_one() {
    debug!("This is ignored!");
    info!("logged at info");
    call_two();
}

#[instrument]
fn call_two() {
    warn!("This is the warning level");
    info!("logged at info");
}
