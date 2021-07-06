use std::time::{Duration, Instant};

use tracing::{info_span, span, Collect};
use tracing_subscriber::{
    self,
    registry::LookupSpan,
    subscribe::{CollectExt, Context},
    util::SubscriberInitExt,
    Subscribe,
};

struct TapSubscriber;

#[derive(Debug)]
struct SpanMetric {
    start_time: Instant,
    metrics: Vec<Metric>,
}

#[derive(Debug)]
struct Metric(&'static str, Duration);

impl SpanMetric {
    fn new() -> Self {
        SpanMetric {
            start_time: Instant::now(),
            metrics: vec![],
        }
    }

    fn record(&mut self, name: &'static str) {
        self.metrics.push(Metric(name, self.start_time.elapsed()));
    }
}

impl<C> Subscribe<C> for TapSubscriber
where
    C: Collect + for<'span> LookupSpan<'span>,
{
    fn new_span(
        &self,
        _attrs: &tracing_core::span::Attributes<'_>,
        id: &tracing_core::span::Id,
        ctx: Context<'_, C>,
    ) {
        if let Some(span) = ctx.span(id) {
            let mut extension = span.extensions_mut();
            extension.insert(SpanMetric::new());
        }
    }

    fn on_tap(&self, id: &span::Id, ctx: Context<'_, C>, value: &dyn span::Tap) {
        if let Some(&name) = value.downcast_ref::<&'static str>() {
            if let Some(span) = ctx.span(id) {
                let mut extension = span.extensions_mut();
                if let Some(metric) = extension.get_mut::<SpanMetric>() {
                    metric.record(name);
                }
            }
        }
    }

    fn on_exit(&self, id: &tracing_core::span::Id, ctx: Context<'_, C>) {
        if let Some(span) = ctx.span(id) {
            let extension = span.extensions();
            if let Some(metric) = extension.get::<SpanMetric>() {
                println!("{:?}", metric);
            }
        }
    }
}

fn main() {
    let tap_subscriber = TapSubscriber;
    tracing_subscriber::registry().with(tap_subscriber).init();

    let span = info_span!("my_span").entered();
    span.tap("fist step");
    span.tap("second step");
    span.tap("third step");
    span.tap("fourth step");
}
