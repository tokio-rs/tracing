use std::{collections::HashMap, fmt, sync::RwLock};
use tracing::{field::Visit, Collect};
use tracing_core::Field;

use opentelemetry::{
    metrics::{Counter, Histogram, Meter, MeterProvider, UpDownCounter},
    sdk::metrics::controllers::BasicController,
    Context as OtelContext, Key, KeyValue, Value,
};
use tracing_subscriber::{registry::LookupSpan, subscribe::Context, Subscribe};

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const INSTRUMENTATION_LIBRARY_NAME: &str = "tracing/tracing-opentelemetry";

const METRIC_PREFIX_MONOTONIC_COUNTER: &str = "monotonic_counter.";
const METRIC_PREFIX_COUNTER: &str = "counter.";
const METRIC_PREFIX_HISTOGRAM: &str = "histogram.";
const I64_MAX: u64 = i64::MAX as u64;

#[derive(Default)]
pub(crate) struct Instruments {
    u64_counter: MetricsMap<Counter<u64>>,
    f64_counter: MetricsMap<Counter<f64>>,
    i64_up_down_counter: MetricsMap<UpDownCounter<i64>>,
    f64_up_down_counter: MetricsMap<UpDownCounter<f64>>,
    u64_histogram: MetricsMap<Histogram<u64>>,
    i64_histogram: MetricsMap<Histogram<i64>>,
    f64_histogram: MetricsMap<Histogram<f64>>,
}

type MetricsMap<T> = RwLock<HashMap<&'static str, T>>;

#[derive(Copy, Clone, Debug)]
pub(crate) enum InstrumentType {
    CounterU64(u64),
    CounterF64(f64),
    UpDownCounterI64(i64),
    UpDownCounterF64(f64),
    HistogramU64(u64),
    HistogramI64(i64),
    HistogramF64(f64),
}

impl Instruments {
    pub(crate) fn update_metric(
        &self,
        cx: &OtelContext,
        meter: &Meter,
        instrument_type: InstrumentType,
        metric_name: &'static str,
        attributes: &[KeyValue],
    ) {
        fn update_or_insert<T>(
            map: &MetricsMap<T>,
            name: &'static str,
            insert: impl FnOnce() -> T,
            update: impl FnOnce(&T),
        ) {
            {
                let lock = map.read().unwrap();
                if let Some(metric) = lock.get(name) {
                    update(metric);
                    return;
                }
            }

            // that metric did not already exist, so we have to acquire a write lock to
            // create it.
            let mut lock = map.write().unwrap();

            // handle the case where the entry was created while we were waiting to
            // acquire the write lock
            let metric = lock.entry(name).or_insert_with(insert);
            update(metric)
        }

        match instrument_type {
            InstrumentType::CounterU64(value) => {
                update_or_insert(
                    &self.u64_counter,
                    metric_name,
                    || meter.u64_counter(metric_name).init(),
                    |ctr| ctr.add(cx, value, attributes),
                );
            }
            InstrumentType::CounterF64(value) => {
                update_or_insert(
                    &self.f64_counter,
                    metric_name,
                    || meter.f64_counter(metric_name).init(),
                    |ctr| ctr.add(cx, value, attributes),
                );
            }
            InstrumentType::UpDownCounterI64(value) => {
                update_or_insert(
                    &self.i64_up_down_counter,
                    metric_name,
                    || meter.i64_up_down_counter(metric_name).init(),
                    |ctr| ctr.add(cx, value, attributes),
                );
            }
            InstrumentType::UpDownCounterF64(value) => {
                update_or_insert(
                    &self.f64_up_down_counter,
                    metric_name,
                    || meter.f64_up_down_counter(metric_name).init(),
                    |ctr| ctr.add(cx, value, attributes),
                );
            }
            InstrumentType::HistogramU64(value) => {
                update_or_insert(
                    &self.u64_histogram,
                    metric_name,
                    || meter.u64_histogram(metric_name).init(),
                    |rec| rec.record(cx, value, attributes),
                );
            }
            InstrumentType::HistogramI64(value) => {
                update_or_insert(
                    &self.i64_histogram,
                    metric_name,
                    || meter.i64_histogram(metric_name).init(),
                    |rec| rec.record(cx, value, attributes),
                );
            }
            InstrumentType::HistogramF64(value) => {
                update_or_insert(
                    &self.f64_histogram,
                    metric_name,
                    || meter.f64_histogram(metric_name).init(),
                    |rec| rec.record(cx, value, attributes),
                );
            }
        };
    }
}

pub(crate) struct MetricVisitor<'a> {
    pub(crate) instruments: &'a Instruments,
    pub(crate) metric: Option<(&'static str, InstrumentType)>,
    pub(crate) attributes: Vec<KeyValue>,
    pub(crate) meter: &'a Meter,
}

impl<'a> Visit for MetricVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        // Do not display the log content
        if field.name() != "message" {
            self.attributes.push(KeyValue::new(
                Key::from_static_str(field.name()),
                Value::from(format!("{:?}", value)),
            ));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.attributes.push(KeyValue::new(
            Key::from_static_str(field.name()),
            Value::from(value.to_string()),
        ));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if let Some(metric_name) = field.name().strip_prefix(METRIC_PREFIX_MONOTONIC_COUNTER) {
            self.metric = Some((metric_name, InstrumentType::CounterU64(value)));
        } else if let Some(metric_name) = field.name().strip_prefix(METRIC_PREFIX_COUNTER) {
            if value <= I64_MAX {
                self.metric = Some((metric_name, InstrumentType::UpDownCounterI64(value as i64)));
            } else {
                eprintln!(
                    "[tracing-opentelemetry]: Received Counter metric, but \
                    provided u64: {} is greater than i64::MAX. Ignoring \
                    this metric.",
                    value
                );
            }
        } else if let Some(metric_name) = field.name().strip_prefix(METRIC_PREFIX_HISTOGRAM) {
            self.metric = Some((metric_name, InstrumentType::HistogramU64(value)));
        } else {
            self.record_debug(field, &value);
        }
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        if let Some(metric_name) = field.name().strip_prefix(METRIC_PREFIX_MONOTONIC_COUNTER) {
            self.metric = Some((metric_name, InstrumentType::CounterF64(value)));
        } else if let Some(metric_name) = field.name().strip_prefix(METRIC_PREFIX_COUNTER) {
            self.metric = Some((metric_name, InstrumentType::UpDownCounterF64(value)));
        } else if let Some(metric_name) = field.name().strip_prefix(METRIC_PREFIX_HISTOGRAM) {
            self.metric = Some((metric_name, InstrumentType::HistogramF64(value)));
        } else {
            self.record_debug(field, &value);
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        if let Some(metric_name) = field.name().strip_prefix(METRIC_PREFIX_MONOTONIC_COUNTER) {
            self.metric = Some((metric_name, InstrumentType::CounterU64(value as u64)));
        } else if let Some(metric_name) = field.name().strip_prefix(METRIC_PREFIX_COUNTER) {
            self.metric = Some((metric_name, InstrumentType::UpDownCounterI64(value)));
        } else if let Some(metric_name) = field.name().strip_prefix(METRIC_PREFIX_HISTOGRAM) {
            self.metric = Some((metric_name, InstrumentType::HistogramI64(value)));
        } else {
            self.record_debug(field, &value);
        }
    }
}

impl<'a> MetricVisitor<'a> {
    fn finish(self) {
        if let Some((metric_name, instrument_type)) = self.metric {
            let cx = OtelContext::current();
            self.instruments.update_metric(
                &cx,
                self.meter,
                instrument_type,
                metric_name,
                &self.attributes,
            );
        }
    }
}

/// A subscriber that publishes metrics via the OpenTelemetry SDK.
///
/// # Usage
///
/// No configuration is needed for this Subscriber, as it's only responsible for
/// pushing data out to the `opentelemetry` family of crates. For example, when
/// using `opentelemetry-otlp`, that crate will provide its own set of
/// configuration options for setting up the duration metrics will be collected
/// before exporting to the OpenTelemetry Collector, aggregation of data points,
/// etc.
///
/// ```no_run
/// use tracing_opentelemetry::MetricsSubscriber;
/// use tracing_subscriber::subscribe::CollectExt;
/// use tracing_subscriber::Registry;
/// # use opentelemetry::sdk::metrics::controllers::BasicController;
///
/// // Constructing a BasicController is out-of-scope for the docs here, but there
/// // are examples in the opentelemetry repository. See:
/// // https://github.com/open-telemetry/opentelemetry-rust/blob/d4b9befea04bcc7fc19319a6ebf5b5070131c486/examples/basic-otlp/src/main.rs#L35-L52
/// # let controller: BasicController = unimplemented!();
///
/// let opentelemetry_metrics =  MetricsSubscriber::new(controller);
/// let collector = Registry::default().with(opentelemetry_metrics);
/// tracing::collect::set_global_default(collector).unwrap();
/// ```
///
/// To publish a new metric, add a key-value pair to your `tracing::Event` that
/// contains following prefixes:
/// - `monotonic_counter.` (non-negative numbers): Used when the counter should
///   only ever increase
/// - `counter.`: Used when the counter can go up or down
/// - `histogram.`: Used for discrete data points (i.e., summing them does not make
///   semantic sense)
///
/// Examples:
/// ```
/// # use tracing::info;
/// info!(monotonic_counter.foo = 1);
/// info!(monotonic_counter.bar = 1.1);
///
/// info!(counter.baz = 1);
/// info!(counter.baz = -1);
/// info!(counter.xyz = 1.1);
///
/// info!(histogram.qux = 1);
/// info!(histogram.abc = -1);
/// info!(histogram.def = 1.1);
/// ```
///
/// # Add attributes on metrics
///
/// Examples:
/// ```
/// # use tracing::info;
/// info!(monotonic_counter.foo = 1, status_code = 404);
/// let kind = "unknown";
/// info!(counter.bar = 1.1, kind);
/// ```
///
/// # Mixing data types
///
/// ## Floating-point numbers
///
/// Do not mix floating point and non-floating point numbers for the same
/// metric. If a floating point number will be used for a given metric, be sure
/// to cast any other usages of that metric to a floating point number.
///
/// Do this:
/// ```
/// # use tracing::info;
/// info!(monotonic_counter.foo = 1_f64);
/// info!(monotonic_counter.foo = 1.1);
/// ```
///
/// This is because all data published for a given metric name must be the same
/// numeric type.
///
/// ## Integers
///
/// Positive and negative integers can be mixed freely. The instrumentation
/// provided by `tracing` assumes that all integers are `i64` unless explicitly
/// cast to something else. In the case that an integer *is* cast to `u64`, this
/// subscriber will handle the conversion internally.
///
/// For example:
/// ```
/// # use tracing::info;
/// // The subscriber receives an i64
/// info!(counter.baz = 1);
///
/// // The subscriber receives an i64
/// info!(counter.baz = -1);
///
/// // The subscriber receives a u64, but casts it to i64 internally
/// info!(counter.baz = 1_u64);
///
/// // The subscriber receives a u64, but cannot cast it to i64 because of
/// // overflow. An error is printed to stderr, and the metric is dropped.
/// info!(counter.baz = (i64::MAX as u64) + 1)
/// ```
///
/// # Implementation Details
///
/// `MetricsSubscriber` holds a set of maps, with each map corresponding to a
/// type of metric supported by OpenTelemetry. These maps are populated lazily.
/// The first time that a metric is emitted by the instrumentation, a `Metric`
/// instance will be created and added to the corresponding map. This means that
/// any time a metric is emitted by the instrumentation, one map lookup has to
/// be performed.
///
/// In the future, this can be improved by associating each `Metric` instance to
/// its callsite, eliminating the need for any maps.
#[cfg_attr(docsrs, doc(cfg(feature = "metrics")))]
pub struct MetricsSubscriber {
    meter: Meter,
    instruments: Instruments,
}

impl MetricsSubscriber {
    /// Create a new instance of MetricsSubscriber.
    pub fn new(controller: BasicController) -> Self {
        let meter =
            controller.versioned_meter(INSTRUMENTATION_LIBRARY_NAME, Some(CARGO_PKG_VERSION), None);
        MetricsSubscriber {
            meter,
            instruments: Default::default(),
        }
    }
}

impl<C> Subscribe<C> for MetricsSubscriber
where
    C: Collect + for<'span> LookupSpan<'span>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, C>) {
        let mut metric_visitor = MetricVisitor {
            instruments: &self.instruments,
            meter: &self.meter,
            metric: None,
            attributes: Vec::new(),
        };
        event.record(&mut metric_visitor);
        metric_visitor.finish();
    }
}
